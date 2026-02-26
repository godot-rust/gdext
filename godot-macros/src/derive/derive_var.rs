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

/// Derives `Var` for the given declaration.
///
/// This uses `ToGodot` and `FromGodot` for the `var_get` and `var_set` implementations.
/// Property hints are derived from `GodotConvert::shape()`.
pub fn derive_var(item: venial::Item) -> ParseResult<TokenStream> {
    let convert = GodotConvert::parse_declaration(item)?;

    let name = convert.ty_name;

    Ok(quote! {
        impl ::godot::register::property::Var for #name {
            type PubType = Self;

            fn var_get(field: &Self) -> <Self as ::godot::meta::GodotConvert>::Via {
                ::godot::meta::ToGodot::to_godot(field)
            }

            fn var_set(field: &mut Self, value: <Self as ::godot::meta::GodotConvert>::Via) {
                *field = ::godot::meta::FromGodot::from_godot(value);
            }

            fn var_pub_get(field: &Self) -> Self::PubType {
                field.clone()
            }

            fn var_pub_set(field: &mut Self, value: Self::PubType) {
                *field = value;
            }
        }
    })
}
