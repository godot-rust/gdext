/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Parsing the `var` and `export` attributes on fields.

use crate::class::{Field, FieldVar, Fields, GetSet, GetterSetterImpl, UsageFlags};
use crate::util;
use proc_macro2::{Ident, TokenStream};
use quote::quote;

#[derive(Default, Clone, Debug)]
pub enum FieldHint {
    /// The hint and hint string should be inferred based on the context.
    #[default]
    Inferred,

    /// Only the hint is explicitly set, the hint string is empty.
    Hint(Ident),

    /// Both hint and hint string are explicitly set.
    HintWithString {
        hint: Ident,
        hint_string: TokenStream,
    },

    /// The hint and hint string are given by a token stream returning an `ExportInfo` struct.
    HintFromExportFunction(TokenStream),
}

impl FieldHint {
    pub fn new(hint: Ident, hint_string: Option<TokenStream>) -> Self {
        match hint_string {
            None => Self::Hint(hint),
            Some(hint_string) => Self::HintWithString { hint, hint_string },
        }
    }
}

pub fn make_property_impl(class_name: &Ident, fields: &Fields) -> TokenStream {
    let class_name_obj = util::class_name_obj(class_name);

    let mut getter_setter_impls = Vec::new();
    let mut export_tokens = Vec::new();

    for field in &fields.all_fields {
        let Field {
            name: field_ident,
            ty: field_type,
            var,
            export,
            ..
        } = field;

        // Ensure we add a var if the user only provided a `#[export]`.
        let var = match (export, var) {
            (Some(_), None) => Some(FieldVar {
                usage_flags: UsageFlags::InferredExport,
                ..Default::default()
            }),

            (_, var) => var.clone(),
        };

        let Some(var) = var else {
            continue;
        };

        let field_variant_type = util::property_variant_type(field_type);
        let field_class_name = util::property_variant_class_name(field_type);
        let field_name = field_ident.to_string();

        // rustfmt wont format this if we put it in the let-else.
        let FieldVar {
            getter,
            setter,
            mut hint,
            mut usage_flags,
        } = var;

        if let Some(export) = export {
            hint = export.to_field_hint();

            if usage_flags.is_inferred() {
                usage_flags = UsageFlags::InferredExport;
            }
        }

        let usage_flags = match usage_flags {
            UsageFlags::Inferred => {
                quote! { ::godot::engine::global::PropertyUsageFlags::PROPERTY_USAGE_NO_EDITOR }
            }
            UsageFlags::InferredExport => {
                quote! { ::godot::engine::global::PropertyUsageFlags::PROPERTY_USAGE_DEFAULT }
            }
            UsageFlags::Custom(flags) => quote! {
                #(
                    ::godot::engine::global::PropertyUsageFlags::#flags
                )|*
            },
        };

        let hint = match hint {
            FieldHint::Inferred => {
                if export.is_some() {
                    quote! {
                        {
                            let default_export_info = <#field_type as ::godot::bind::property::Export>::default_export_info();
                            (default_export_info.hint, default_export_info.hint_string)
                        }
                    }
                } else {
                    quote! {
                        {
                            (
                                ::godot::engine::global::PropertyHint::PROPERTY_HINT_NONE,
                                ::godot::builtin::GodotString::new()
                            )
                        }
                    }
                }
            }
            FieldHint::Hint(hint) => quote! {
                (
                    ::godot::engine::global::PropertyHint::#hint,
                    ::godot::builtin::GodotString::new()
                )
            },
            FieldHint::HintWithString { hint, hint_string } => quote! {
                (
                    ::godot::engine::global::PropertyHint::#hint,
                    ::godot::builtin::GodotString::from(#hint_string)
                )
            },
            FieldHint::HintFromExportFunction(expression) => quote! {
                {
                    let ::godot::bind::property::ExportInfo { hint, hint_string } = #expression;
                    (hint, hint_string)
                }
            },
        };

        let getter_name = if let Some(getter_impl) = getter.to_impl(class_name, GetSet::Get, field)
        {
            let GetterSetterImpl {
                function_name,
                function_impl,
                export_token,
            } = getter_impl;

            getter_setter_impls.push(function_impl);
            export_tokens.push(export_token);

            function_name.to_string()
        } else {
            String::new()
        };

        let setter_name = if let Some(setter_impl) = setter.to_impl(class_name, GetSet::Set, field)
        {
            let GetterSetterImpl {
                function_name,
                function_impl,
                export_token,
            } = setter_impl;

            getter_setter_impls.push(function_impl);
            export_tokens.push(export_token);

            function_name.to_string()
        } else {
            String::new()
        };

        export_tokens.push(quote! {
            use ::godot::builtin::meta::VariantMetadata;

            let (hint, hint_string) = #hint;
            let usage = #usage_flags;

            let property_info = ::godot::builtin::meta::PropertyInfo {
                variant_type: #field_variant_type,
                class_name: #field_class_name,
                property_name: #field_name.into(),
                hint,
                hint_string,
                usage,
            };

            let getter_name = ::godot::builtin::StringName::from(#getter_name);
            let setter_name = ::godot::builtin::StringName::from(#setter_name);

            let property_info_sys = property_info.property_sys();

            unsafe {
                ::godot::sys::interface_fn!(classdb_register_extension_class_property)(
                    ::godot::sys::get_library(),
                    #class_name_obj.string_sys(),
                    std::ptr::addr_of!(property_info_sys),
                    setter_name.string_sys(),
                    getter_name.string_sys(),
                );
            }
        });
    }

    let enforce_godot_api_impl = if !export_tokens.is_empty() {
        quote! {
            const MUST_HAVE_GODOT_API_IMPL: () = <#class_name as ::godot::private::Cannot_export_without_godot_api_impl>::EXISTS;
        }
    } else {
        TokenStream::new()
    };

    quote! {
        impl #class_name {
            #enforce_godot_api_impl

            #(#getter_setter_impls)*
        }

        impl ::godot::obj::cap::ImplementsGodotExports for #class_name {
            fn __register_exports() {
                #(
                    {
                        #export_tokens
                    }
                )*
            }
        }
    }
}
