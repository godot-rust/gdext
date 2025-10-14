/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::collections::HashMap;

use proc_macro2::{Ident, TokenStream, TokenTree};
use quote::quote;

use crate::derive::data_models::{ConvertType, GodotConvert};
use crate::derive::{make_fromgodot, make_togodot};
use crate::ParseResult;

/// Derives `GodotConvert` for the given declaration.
///
/// This also derives `FromGodot` and `ToGodot`.
pub fn derive_godot_convert(item: venial::Item) -> ParseResult<TokenStream> {
    let convert = GodotConvert::parse_declaration(item)?;

    let name = &convert.ty_name;
    let via_type = convert.convert_type.via_type();
    let mut cache = EnumeratorExprCache::default();

    let to_godot_impl = make_togodot(&convert, &mut cache);
    let from_godot_impl = make_fromgodot(&convert, &mut cache);

    // Generate enumerator metadata and registration for enums
    let enum_extras = match &convert.convert_type {
        ConvertType::Enum {
            variants,
            class,
            via: _,
        } => {
            let enumerator_names = variants.enumerator_names();
            let enumerator_ords = variants.enumerator_ord_exprs();

            // Convert ordinal expressions to i64 casts for the metadata
            let enumerator_name_strs: Vec<_> =
                enumerator_names.iter().map(|n| n.to_string()).collect();
            let enumerator_values: Vec<_> = enumerator_ords
                .iter()
                .map(|ord| quote! { #ord as i64 })
                .collect();

            // Generate metadata constants
            let metadata = quote! {
                impl #name {
                    #[doc(hidden)]
                    pub const __GODOT_ENUMERATOR_NAMES: &'static [&'static str] = &[
                        #( #enumerator_name_strs ),*
                    ];

                    #[doc(hidden)]
                    pub const __GODOT_ENUMERATOR_VALUES: &'static [i64] = &[
                        #( #enumerator_values ),*
                    ];
                }
            };

            // Generate auto-registration code
            let registration = generate_enum_registration(name, class)?;

            quote! {
                #metadata
                #registration
            }
        }
        ConvertType::NewType { .. } => TokenStream::new(),
    };

    Ok(quote! {
        impl ::godot::meta::GodotConvert for #name  {
            type Via = #via_type;
        }

        #to_godot_impl
        #from_godot_impl
        #enum_extras
    })
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Helpers for submodules

/// Caches enumerator ordinal expressions that are modified, e.g. `(1 + 2) as isize` -> `(1 + 2) as i64`.
#[derive(Default)]
pub struct EnumeratorExprCache {
    is_initialized: bool,
    /// Contains only overridden ones (where the default wouldn't fit). Key is enumerator name.
    ord_expr_by_name: HashMap<Ident, TokenStream>,
}

impl EnumeratorExprCache {
    /// Returns an iterator of ord expressions, with those replaced that have been overridden.
    ///
    /// Requires that parameters are the same as in previous calls.
    pub fn map_ord_exprs<'ords: 'cache, 'cache>(
        &'cache mut self,
        int: &'ords Ident,
        names: &'ords [Ident],
        ord_exprs: &'ords [TokenStream],
    ) -> impl Iterator<Item = &'cache TokenStream> + 'cache {
        self.ensure_initialized(int, names, ord_exprs);

        names
            .iter()
            .zip(ord_exprs.iter())
            .map(|(name, ord_expr)| self.ord_expr_by_name.get(name).unwrap_or(ord_expr))
    }

    /// Goes through all (name, ord_expr) pairs and builds special cases.
    ///
    /// If initialized before, does nothing.
    fn ensure_initialized(&mut self, int: &Ident, names: &[Ident], ord_exprs: &[TokenStream]) {
        if self.is_initialized {
            return;
        }

        for (enumerator_name, ord_expr) in names.iter().zip(ord_exprs) {
            if let Some(new_ord_expr) = adjust_ord_expr(ord_expr, int) {
                self.ord_expr_by_name
                    .insert(enumerator_name.clone(), new_ord_expr);
            }
        }

        self.is_initialized = true;
    }
}

fn adjust_ord_expr(ord_expr: &TokenStream, int: &Ident) -> Option<TokenStream> {
    // If the token stream ends in `as isize`, this is typically a constant conversion (e.g. MyVariant = OtherEnum::Variant as isize).
    // Then, replace `as isize` (which is required for Rust enum) with `as #int`. This currently ignores type narrowing errors.

    let paren_group = ord_expr
        .clone()
        .into_iter()
        .next()
        .expect("no tokens in enumerator ord expression");

    let TokenTree::Group(paren_expr) = paren_group else {
        // Early exit for simple expressions (literals).
        return None;
    };

    // Could technically save this allocation by using field + clear() + extend().
    let mut tokens = Vec::from_iter(paren_expr.stream());

    match tokens.as_slice() {
        // Ends with `as isize` => likely using another constant. We replace it with `as #int`, so it fits the underlying Godot type.
        // Since this is a derive macro, we can unfortunately not change the original definition.
        [.., TokenTree::Ident(tk_as), TokenTree::Ident(tk_isize)]
            if tk_as == "as" && tk_isize == "isize" =>
        {
            tokens.pop();
            tokens.push(TokenTree::Ident(int.clone()));

            let stream = TokenStream::from_iter(tokens.iter().cloned());
            Some(stream)
        }
        _ => None,
    }
}

/// Generates auto-registration code for enum constants.
fn generate_enum_registration(
    enum_name: &Ident,
    class_type: &Option<venial::TypeExpr>,
) -> ParseResult<TokenStream> {
    use quote::ToTokens;

    let enum_name_str = enum_name.to_string();
    let registration_struct = quote::format_ident!("__EnumRegistration_{}", enum_name);

    if let Some(class_ty) = class_type {
        // Class-scoped enum: register to the class
        let class_name = class_ty.to_token_stream();

        Ok(quote! {
            // Create a GodotClass for registration (internal, not exposed)
            #[doc(hidden)]
            #[derive(::godot::prelude::GodotClass)]
            #[class(no_init, internal)]
            struct #registration_struct;

            #[::godot::prelude::godot_api]
            impl #registration_struct {
                #[func]
                fn __register_enum_constants() {
                    use ::godot::register::private::constant::*;

                    let mut enumerators = Vec::new();
                    for i in 0..#enum_name::__GODOT_ENUMERATOR_NAMES.len() {
                        enumerators.push(IntegerConstant::new(
                            #enum_name::__GODOT_ENUMERATOR_NAMES[i],
                            #enum_name::__GODOT_ENUMERATOR_VALUES[i],
                        ));
                    }

                    ExportConstant::new(
                        <#class_name as ::godot::obj::GodotClass>::class_id(),
                        ConstantKind::Enum {
                            name: #enum_name_str.into(),
                            enumerators,
                        },
                    ).register();
                }
            }

            // Call the registration during initialization
            const _: () = {
                ::godot::sys::plugin_execute_pre_main!({
                    #registration_struct::__register_enum_constants();
                });
            };
        })
    } else {
        // Global enum: register to ClassId::none()
        Ok(quote! {
            // Create a GodotClass for registration (internal, not exposed)
            #[doc(hidden)]
            #[derive(::godot::prelude::GodotClass)]
            #[class(no_init, internal)]
            struct #registration_struct;

            impl ::godot::obj::cap::ImplementsGodotApi for #registration_struct {
                fn __register_methods() {}

                fn __register_constants() {
                    use ::godot::register::private::constant::*;
                    use ::godot::meta::ClassId;

                    let mut enumerators = Vec::new();
                    for i in 0..#enum_name::__GODOT_ENUMERATOR_NAMES.len() {
                        enumerators.push(IntegerConstant::new(
                            #enum_name::__GODOT_ENUMERATOR_NAMES[i],
                            #enum_name::__GODOT_ENUMERATOR_VALUES[i],
                        ));
                    }

                    ExportConstant::new(
                        ClassId::none(),
                        ConstantKind::Enum {
                            name: #enum_name_str.into(),
                            enumerators,
                        },
                    ).register();
                }
            }

            // Register using the plugin system (same as manual registration in tests)
            ::godot::sys::plugin_add!(
                ::godot::private::__GODOT_PLUGIN_REGISTRY;
                ::godot::private::ClassPlugin::new::<#registration_struct>(
                    ::godot::private::PluginItem::InherentImpl(
                        ::godot::private::InherentImpl::new::<#registration_struct>(
                            #[cfg(feature = "register-docs")]
                            Default::default()
                        )
                    )
                )
            );
        })
    }
}
