/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::collections::HashMap;

use proc_macro2::{Ident, TokenStream, TokenTree};
use quote::quote;

use crate::derive::data_models::GodotConvert;
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

    Ok(quote! {
        impl ::godot::meta::GodotConvert for #name  {
            type Via = #via_type;
        }

        #to_godot_impl
        #from_godot_impl
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
