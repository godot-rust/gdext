/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use proc_macro2::TokenStream;
use quote::quote;

use crate::derive::data_models::GodotConvert;
use crate::ParseResult;

/// Derives `Var` for the given declaration.
///
/// This uses `ToGodot` and `FromGodot` for the `var_get` and `var_set` implementations.
pub fn derive_var(item: venial::Item) -> ParseResult<TokenStream> {
    let convert = GodotConvert::parse_declaration(item)?;

    let property_hint_impl = create_property_hint_impl(&convert);

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

            fn var_hint() -> ::godot::meta::PropertyHintInfo {
                #property_hint_impl
            }
        }
    })
}

/// Make an appropriate property hint implementation.
///
/// For newtype structs we just defer to the wrapped type. For enums we use `PropertyHint::ENUM` with an appropriate hint string.
fn create_property_hint_impl(convert: &GodotConvert) -> TokenStream {
    use super::data_models::{ConvertType, ViaType};

    match &convert.convert_type {
        ConvertType::NewType { field } => {
            let ty = &field.ty;
            quote! {
                <#ty as ::godot::register::property::Var>::var_hint()
            }
        }
        ConvertType::Enum { variants, via } => {
            let hint_string = match via {
                ViaType::GString { .. } => variants.to_string_hint(),
                ViaType::Int { .. } => variants.to_int_hint(),
            };

            quote! {
                ::godot::meta::PropertyHintInfo {
                    hint: ::godot::global::PropertyHint::ENUM,
                    hint_string: ::godot::builtin::GString::from(#hint_string),
                }
            }
        }
    }
}
