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

pub fn derive_from_godot(declaration: Declaration) -> ParseResult<TokenStream> {
    let GodotConvert { name, data } = GodotConvert::parse_declaration(declaration)?;

    match data {
        ConvertData::NewType { field } => from_newtype(name, field),
        ConvertData::Enum {
            variants,
            via: ViaType::GString(_),
        } => from_enum_string(name, variants),
        ConvertData::Enum {
            variants,
            via: ViaType::Int(_, int),
        } => from_enum_int(name, variants, int.to_ident()),
    }
}

fn from_newtype(name: Ident, field: NewtypeField) -> ParseResult<TokenStream> {
    // For tuple structs this ends up using the alternate tuple-struct constructor syntax of
    // TupleStruct { .0: value }
    let field_name = field.field_name();
    let via_type = field.ty;

    Ok(quote! {
        impl ::godot::builtin::meta::FromGodot for #name {
            fn try_from_godot(via: #via_type) -> ::std::result::Result<Self, ::godot::builtin::meta::ConvertError> {
                Ok(Self { #field_name: via })
            }
        }
    })
}

fn from_enum_int(name: Ident, enum_: CStyleEnum, int: Ident) -> ParseResult<TokenStream> {
    let discriminants = enum_
        .discriminants()
        .iter()
        .map(|i| Literal::i64_unsuffixed(*i))
        .collect::<Vec<_>>();
    let names = enum_.names();
    let bad_variant_error = format!("invalid {name} variant");

    Ok(quote! {
        impl ::godot::builtin::meta::FromGodot for #name {
            fn try_from_godot(via: #int) -> ::std::result::Result<Self, ::godot::builtin::meta::ConvertError> {
                match via {
                    #(
                        #discriminants => Ok(#name::#names),
                    )*
                    other => Err(::godot::builtin::meta::ConvertError::with_cause_value(#bad_variant_error, other))
                }
            }
        }
    })
}

fn from_enum_string(name: Ident, enum_: CStyleEnum) -> ParseResult<TokenStream> {
    let names = enum_.names();
    let names_str = names.iter().map(ToString::to_string).collect::<Vec<_>>();
    let bad_variant_error = format!("invalid {name} variant");

    Ok(quote! {
        impl ::godot::builtin::meta::FromGodot for #name {
            fn try_from_godot(via: ::godot::builtin::GString) -> ::std::result::Result<Self, ::godot::builtin::meta::ConvertError> {
                match via.to_string().as_str() {
                    #(
                        #names_str => Ok(#name::#names),
                    )*
                    other => Err(::godot::builtin::meta::ConvertError::with_cause_value(#bad_variant_error, other))
                }
            }
        }
    })
}
