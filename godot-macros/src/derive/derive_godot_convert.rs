/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

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

    let to_godot_impl = make_togodot(&convert);
    let from_godot_impl = make_fromgodot(&convert);

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

/// Replaces some enumerator ord expressions ending with `as isize`, so they can be assigned to the underlying Godot Via type.
pub(super) fn adjust_ord_exprs(ord_exprs: &[TokenStream], int: &Ident) -> Vec<TokenStream> {
    // Optimization note: clones and allocates a lot. If causing problems, code can be made more complex to reuse memory.

    let mut tokens = vec![]; // Reuse to avoid even more allocs.

    ord_exprs
        .iter()
        .map(|expr| adjust_ord_expr(expr, int, &mut tokens))
        .collect()
}

fn adjust_ord_expr(
    ord_expr: &TokenStream,
    int: &Ident,
    tokens: &mut Vec<TokenTree>,
) -> TokenStream {
    // If the token stream ends in `as isize`, this is typically a constant conversion (e.g. MyVariant = OtherEnum::Variant as isize).
    // Then, replace `as isize` (which is required for Rust enum) with `as #int`. This currently ignores type narrowing errors.

    let paren_group = ord_expr
        .clone()
        .into_iter()
        .next()
        .expect("no tokens in enumerator ord expression");

    let TokenTree::Group(paren_expr) = paren_group else {
        // Early exit for simple expressions (literals).
        return ord_expr.clone();
    };

    tokens.clear();
    tokens.extend(paren_expr.stream());

    match tokens.as_slice() {
        // Ends with `as isize` => likely using another constant. We replace it with `as #int`, so it fits the underlying Godot type.
        // Since this is a derive macro, we can unfortunately not change the original definition.
        [.., TokenTree::Ident(tk_as), TokenTree::Ident(tk_isize)] => {
            if tk_as == "as" && tk_isize == "isize" {
                tokens.pop();
                tokens.push(TokenTree::Ident(int.clone()));
                return TokenStream::from_iter(tokens.iter().cloned());
            }
        }
        _ => return ord_expr.clone(),
    }

    ord_expr.clone()
}
