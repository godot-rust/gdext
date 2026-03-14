/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::collections::HashMap;

use proc_macro2::{Ident, TokenStream, TokenTree};
use quote::quote;

use crate::ParseResult;
use crate::derive::data_models::{ConvertType, GodotConvert, ViaType};
use crate::derive::{make_fromgodot, make_togodot};

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

    let shape_override = make_shape_override(&convert.convert_type, &mut cache);

    Ok(quote! {
        impl ::godot::meta::GodotConvert for #name  {
            type Via = #via_type;
            #shape_override
        }

        #to_godot_impl
        #from_godot_impl

        // Marker impl: defaults derive element metadata from shape().
        impl ::godot::meta::Element for #name {}
    })
}

/// Generates `shape()` override for enum types. Newtypes return `Builtin` (the default), so no override needed.
fn make_shape_override(convert_type: &ConvertType, cache: &mut EnumeratorExprCache) -> TokenStream {
    match convert_type {
        ConvertType::Enum { variants, via } => {
            let names = variants.enumerator_names();

            let enumerator_entries: Vec<TokenStream> = match via {
                ViaType::Int { int_ident, .. } => {
                    // Int-backed enum: EnumeratorShape::new_int("Grass", <ord> as i64).
                    let ord_exprs = variants.enumerator_ord_exprs();
                    let mapped = cache.map_ord_exprs(int_ident, names, ord_exprs);
                    names
                        .iter()
                        .zip(mapped)
                        .map(|(ident, ord_expr)| {
                            let name_str = ident.to_string();
                            quote! {
                                ::godot::meta::shape::EnumeratorShape::new_int(#name_str, #ord_expr as i64)
                            }
                        })
                        .collect()
                }
                ViaType::GString { .. } => {
                    // String-backed enum: EnumeratorShape::new_string("Grass").
                    names
                        .iter()
                        .map(|ident| {
                            let name_str = ident.to_string();
                            quote! {
                                ::godot::meta::shape::EnumeratorShape::new_string(#name_str)
                            }
                        })
                        .collect()
                }
            };

            quote! {
                fn godot_shape() -> ::godot::meta::shape::GodotShape {
                    // Rust enum discriminants are always const expressions, so this works even for `MyVariant = OTHER_CONST as isize`.
                    const ENUMERATORS: &[::godot::meta::shape::EnumeratorShape] = &[
                        #( #enumerator_entries ),*
                    ];
                    ::godot::meta::shape::GodotShape::Enum {
                        variant_type: ::godot::meta::element_variant_type::<Self>(),
                        enumerators: std::borrow::Cow::Borrowed(ENUMERATORS),
                        godot_name: None, // User enums have no Godot class_name (future: register via classdb FFI).
                        is_bitfield: false,
                    }
                }
            }
        }
        ConvertType::NewType { .. } => {
            // Newtypes delegate to the Via type's shape.
            quote! {
                fn godot_shape() -> ::godot::meta::shape::GodotShape {
                    <Self::Via as ::godot::meta::GodotConvert>::godot_shape()
                }
            }
        }
    }
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
