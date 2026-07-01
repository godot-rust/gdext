/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use proc_macro2::TokenStream;
use quote::quote;

use crate::ParseResult;
use crate::derive::data_models::GodotConvert;

/// Derives `Export` for the declaration.
///
/// This currently just reuses the property hint from the `Var` implementation.
pub fn derive_export(item: venial::Item) -> ParseResult<TokenStream> {
    let GodotConvert {
        ty_name: name,
        where_clause,
        generic_params,
        ..
    } = GodotConvert::parse_declaration(item)?;

    let generic_args = generic_params
        .as_ref()
        .map(|params| params.as_inline_args());

    Ok(quote! {
        impl #generic_params ::godot::register::property::Export for #name #generic_args #where_clause {}
    })
}
