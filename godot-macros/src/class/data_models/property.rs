/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Parses the `#[var]` and `#[export]` attributes on fields.

use crate::class::data_models::fields::Fields;
use crate::class::data_models::group_export::FieldGroup;
use crate::class::{Field, FieldVar, GetSet, GetterSetterImpl, UsageFlags};
use crate::util::{format_funcs_collection_constant, format_funcs_collection_struct, ident};
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
    let mut getter_setter_impls = Vec::new();
    let mut func_name_consts = Vec::new();
    let mut export_tokens = Vec::new();

    let (mut current_group, mut current_subgroup) = (None, None);

    for field in &fields.all_fields {
        let Field {
            name: field_ident,
            ty: field_type,
            var,
            export,
            group,
            ..
        } = field;

        // Ensure we add a var if the user only provided a `#[export]`.
        let var = match (export, var) {
            (Some(export), None) => {
                let usage_flags = if let Some(usage) = export.to_export_usage() {
                    UsageFlags::Custom(vec![usage])
                } else {
                    UsageFlags::InferredExport
                };
                FieldVar {
                    usage_flags,
                    ..Default::default()
                }
            }

            (_, Some(var)) => var.clone(),
            _ => continue,
        };

        register_group_or_subgroup(
            group,
            &mut current_group,
            &mut current_subgroup,
            &fields.groups,
            &mut export_tokens,
            class_name,
        );

        let field_name = field_ident.to_string();

        let FieldVar {
            getter,
            setter,
            hint,
            mut usage_flags,
            ..
        } = var;

        let export_hint;
        let registration_fn;

        if let Some(export) = export {
            if usage_flags.is_inferred() {
                usage_flags = UsageFlags::InferredExport;
            }

            export_hint = export.to_export_hint();
            registration_fn = quote! { register_export };
        } else {
            export_hint = None;
            registration_fn = quote! { register_var };
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
                    quote! { #export_hint }
                } else if export.is_some() {
                    quote! { <#field_type as ::godot::register::property::Export>::export_hint() }
                } else {
                    quote! { <#field_type as ::godot::register::property::Var>::var_hint() }
                }
            }
            FieldHint::Hint(hint) => {
                let hint_string = if let Some(export_hint) = export_hint {
                    quote! { #export_hint.hint_string }
                } else {
                    quote! { ::godot::builtin::GString::new() }
                };

                quote! {
                    ::godot::meta::PropertyHintInfo {
                        hint: ::godot::global::PropertyHint::#hint,
                        hint_string: #hint_string,
                    }
                }
            }
            FieldHint::HintWithString { hint, hint_string } => quote! {
                ::godot::meta::PropertyHintInfo {
                    hint: ::godot::global::PropertyHint::#hint,
                    hint_string: ::godot::builtin::GString::from(#hint_string),
                }
            },
        };

        // Note: {getter,setter}_tokens can be either a path `Class_Functions::constant_name` or an empty string `""`.

        let getter_tokens = make_getter_setter(
            getter.to_impl(class_name, GetSet::Get, field),
            &mut getter_setter_impls,
            &mut func_name_consts,
            &mut export_tokens,
            class_name,
        );
        let setter_tokens = make_getter_setter(
            setter.to_impl(class_name, GetSet::Set, field),
            &mut getter_setter_impls,
            &mut func_name_consts,
            &mut export_tokens,
            class_name,
        );

        export_tokens.push(quote! {
            // This type may be reused in #hint, in case of generic functions.
            type FieldType = #field_type;
            ::godot::register::private::#registration_fn::<#class_name, FieldType>(
                #field_name,
                #getter_tokens,
                #setter_tokens,
                #hint,
                #usage_flags,
            );
        });
    }

    // For each generated #[func], add a const declaration.
    // This is the name of the container struct, which is declared by #[derive(GodotClass)].
    let class_functions_name = format_funcs_collection_struct(class_name);

    quote! {
        impl #class_name {
            #(#getter_setter_impls)*
        }

        impl #class_functions_name {
            #(#func_name_consts)*
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
    func_name_consts: &mut Vec<TokenStream>,
    export_tokens: &mut Vec<TokenStream>,
    class_name: &Ident,
) -> TokenStream {
    let Some(gs) = getter_setter_impl else {
        return quote! { "" };
    };

    getter_setter_impls.push(gs.function_impl);
    func_name_consts.push(gs.funcs_collection_constant);
    export_tokens.push(gs.export_token);

    // Getters/setters are, like #[func]s, subject to additional code generation: a constant inside a "funcs collection" struct
    // stores their Godot name and can be used as an indirection to refer to their true name from other procedural macros.
    let funcs_collection = format_funcs_collection_struct(class_name);
    let constant = format_funcs_collection_constant(class_name, &gs.function_name);

    quote! { #funcs_collection::#constant }
}

/// Pushes registration of group or subgroup in case if they differ from previously registered ones.
///
/// Group membership of properties is based on the order of their registration.
/// All the properties belong to group or subgroup registered beforehand.
fn register_group_or_subgroup(
    field_group: &Option<FieldGroup>,
    current_group: &mut Option<usize>,
    current_subgroup: &mut Option<usize>,
    groups: &[String],
    export_tokens: &mut Vec<TokenStream>,
    class_name: &Ident,
) {
    let Some(group) = field_group else {
        return;
    };

    if let Some(group_registration) = make_group_registration(
        current_group,
        group.group_name_index,
        groups,
        ident("register_group"),
        class_name,
    ) {
        export_tokens.push(group_registration);
    }

    if let Some(subgroup_registration) = make_group_registration(
        current_subgroup,
        group.subgroup_name_index,
        groups,
        ident("register_subgroup"),
        class_name,
    ) {
        export_tokens.push(subgroup_registration);
    }
}

fn make_group_registration(
    current: &mut Option<usize>,
    new: Option<usize>,
    groups: &[String],
    register_fn: Ident,
    class_name: &Ident,
) -> Option<TokenStream> {
    let new_group = new?;

    if current.is_none_or(|cur| cur != new_group) {
        *current = Some(new_group);
        let group = groups.get(new_group)?;
        Some(quote! {
        ::godot::register::private::#register_fn::<#class_name>(
                        #group,
                    );
        })
    } else {
        None
    }
}
