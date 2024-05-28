/*
 * Copyright (c) godot-rust; Bromeon and contributors.
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

        let FieldVar {
            getter,
            setter,
            hint,
            mut usage_flags,
        } = var;

        let mut export_hint = None;

        if let Some(export) = export {
            export_hint = export.to_export_hint();

            if usage_flags.is_inferred() {
                usage_flags = UsageFlags::InferredExport;
            }
        }

        let usage_flags = match usage_flags {
            UsageFlags::Inferred => {
                quote! { ::godot::global::PropertyUsageFlags::NONE }
            }
            UsageFlags::InferredExport => {
                quote! { ::godot::global::PropertyUsageFlags::DEFAULT }
            }
            UsageFlags::Custom(flags) => quote! {
                #(
                    ::godot::global::PropertyUsageFlags::#flags
                )|*
            },
        };

        let hint = match hint {
            FieldHint::Inferred => {
                if let Some(export_hint) = export_hint {
                    quote! {
                        {
                            let ::godot::register::property::PropertyHintInfo { hint, hint_string } = #export_hint;
                            (hint, hint_string)
                        }
                    }
                } else if export.is_some() {
                    quote! {
                        {
                            let default_export_info = <#field_type as ::godot::register::property::Export>::default_export_info();
                            (default_export_info.hint, default_export_info.hint_string)
                        }
                    }
                } else {
                    quote! {
                        {
                            let default_export_info = <#field_type as ::godot::register::property::Var>::property_hint();
                            (default_export_info.hint, default_export_info.hint_string)
                        }
                    }
                }
            }
            FieldHint::Hint(hint) => {
                let hint_string = if let Some(export_hint) = export_hint {
                    quote! { #export_hint.hint_string }
                } else {
                    quote! { :godot::builtin::GString::new() }
                };

                quote! {
                    (
                        ::godot::global::PropertyHint::#hint,
                        #hint_string
                    )
                }
            }
            FieldHint::HintWithString { hint, hint_string } => quote! {
                (
                    ::godot::global::PropertyHint::#hint,
                    ::godot::builtin::GString::from(#hint_string)
                )
            },
        };

        let getter_name = make_getter_setter(
            getter.to_impl(class_name, GetSet::Get, field),
            &mut getter_setter_impls,
            &mut export_tokens,
        );
        let setter_name = make_getter_setter(
            setter.to_impl(class_name, GetSet::Set, field),
            &mut getter_setter_impls,
            &mut export_tokens,
        );

        export_tokens.push(quote! {
            use ::godot::sys::GodotFfi;

            let (hint, hint_string) = #hint;
            let usage = #usage_flags;

            let property_info = ::godot::meta::PropertyInfo {
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

    quote! {
        impl #class_name {
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

fn make_getter_setter(
    getter_setter_impl: Option<GetterSetterImpl>,
    getter_setter_impls: &mut Vec<TokenStream>,
    export_tokens: &mut Vec<TokenStream>,
) -> String {
    if let Some(getter_impl) = getter_setter_impl {
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
    }
}
