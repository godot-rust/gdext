/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use proc_macro2::{Ident, Literal, TokenStream};
use quote::quote;
use venial::Declaration;

use crate::derive::data_model::{ConvertData, GodotConvert, ViaType};
use crate::ParseResult;

use super::data_model::{CStyleEnum, NewtypeField};

pub fn derive_to_godot(declaration: Declaration) -> ParseResult<TokenStream> {
    let GodotConvert { name, data } = GodotConvert::parse_declaration(declaration)?;

    match data {
        ConvertData::NewType { field } => to_newtype(name, field),
        ConvertData::Enum {
            variants,
            via: ViaType::GString(_),
        } => to_enum_string(name, variants),
        ConvertData::Enum {
            variants,
            via: ViaType::Int(_, int),
        } => to_enum_int(name, variants, int.to_ident()),
    }
}

fn to_newtype(name: Ident, field: NewtypeField) -> ParseResult<TokenStream> {
    let field_name = field.field_name();
    let via_type = field.ty;

    Ok(quote! {
        impl ::godot::builtin::meta::ToGodot for #name {
            fn to_godot(&self) -> #via_type {
                ::godot::builtin::meta::ToGodot::to_godot(&self.#field_name)
            }

            fn into_godot(self) -> #via_type {
                ::godot::builtin::meta::ToGodot::into_godot(self.#field_name)
            }
        }
    })
}

fn to_enum_int(name: Ident, enum_: CStyleEnum, int: Ident) -> ParseResult<TokenStream> {
    let discriminants = enum_
        .discriminants()
        .iter()
        .map(|i| Literal::i64_unsuffixed(*i))
        .collect::<Vec<_>>();
    let names = enum_.names();

    Ok(quote! {
        impl ::godot::builtin::meta::ToGodot for #name {
            fn to_godot(&self) -> #int {
                match self {
                    #(
                        #name::#names => #discriminants,
                    )*
                }
            }
        }
    })
}

fn to_enum_string(name: Ident, enum_: CStyleEnum) -> ParseResult<TokenStream> {
    let names = enum_.names();
    let names_str = names.iter().map(ToString::to_string).collect::<Vec<_>>();

    Ok(quote! {
        impl ::godot::builtin::meta::ToGodot for #name {
            fn to_godot(&self) -> ::godot::builtin::GString {
                match self {
                    #(
                        #name::#names => #names_str.into(),
                    )*
                }
            }
        }
    })
}
