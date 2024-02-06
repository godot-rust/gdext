/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use proc_macro2::TokenStream;
use quote::quote;
use venial::Declaration;

use crate::ParseResult;

use super::data_model::GodotConvert;

pub fn derive_var(declaration: Declaration) -> ParseResult<TokenStream> {
    let convert = GodotConvert::parse_declaration(declaration)?;

    let property_hint_impl = create_property_hint_impl(&convert);

    let name = convert.name;

    Ok(quote! {
        impl ::godot::register::property::Var for #name {
            fn get_property(&self) -> <Self as ::godot::builtin::meta::GodotConvert>::Via {
                ::godot::builtin::meta::ToGodot::to_godot(self)
            }

            fn set_property(&mut self, value: <Self as ::godot::builtin::meta::GodotConvert>::Via) {
                *self = ::godot::builtin::meta::FromGodot::from_godot(value);
            }

            fn property_hint() -> ::godot::register::property::PropertyHintInfo {
                #property_hint_impl
            }

        }
    })
}

fn create_property_hint_impl(convert: &GodotConvert) -> TokenStream {
    use super::data_model::ConvertData as Data;
    use super::data_model::ViaType;

    match &convert.data {
        Data::NewType { field } => {
            let ty = &field.ty;
            quote! {
                <#ty as ::godot::register::property::Var>::property_hint()
            }
        }
        Data::Enum { variants, via } => {
            let hint_string = match via {
                ViaType::GString(_) => variants.to_string_hint(),
                ViaType::Int(_, _) => variants.to_int_hint(),
            };

            quote! {
                ::godot::register::property::PropertyHintInfo {
                    hint: ::godot::engine::global::PropertyHint::ENUM,
                    hint_string: #hint_string.into(),
                }
            }
        }
    }
}
