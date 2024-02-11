/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use proc_macro2::{Ident, TokenStream};
use quote::quote;

use crate::derive::data_models::{CStyleEnum, ConvertType, GodotConvert, NewtypeStruct, ViaType};

/// Creates a `FromGodot` impl for the given `GodotConvert`.
///
/// There is no dedicated `FromGodot` derive macro currently, this is instead called by the `GodotConvert` derive macro.
pub fn make_fromgodot(convert: &GodotConvert) -> TokenStream {
    let GodotConvert {
        ty_name: name,
        convert_type: data,
    } = convert;

    match data {
        ConvertType::NewType { field } => make_fromgodot_for_newtype_struct(name, field),
        ConvertType::Enum {
            variants,
            via: ViaType::GString { .. },
        } => make_fromgodot_for_gstring_enum(name, variants),
        ConvertType::Enum {
            variants,
            via: ViaType::Int { int_ident },
        } => make_fromgodot_for_int_enum(name, variants, int_ident),
    }
}

/// Derives `FromGodot` for newtype structs.
fn make_fromgodot_for_newtype_struct(name: &Ident, field: &NewtypeStruct) -> TokenStream {
    // For tuple structs this ends up using the alternate tuple-struct constructor syntax of
    // TupleStruct { 0: value }
    let field_name = field.field_name();
    let via_type = &field.ty;

    quote! {
        impl ::godot::builtin::meta::FromGodot for #name {
            fn try_from_godot(via: #via_type) -> ::std::result::Result<Self, ::godot::builtin::meta::ConvertError> {
                Ok(Self { #field_name: via })
            }
        }
    }
}

/// Derives `FromGodot` for enums with a via type of integers.
fn make_fromgodot_for_int_enum(name: &Ident, enum_: &CStyleEnum, int: &Ident) -> TokenStream {
    let discriminants = enum_.discriminants();
    let names = enum_.names();
    let bad_variant_error = format!("invalid {name} variant");

    quote! {
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
    }
}

/// Derives `FromGodot` for enums with a via type of `GString`.
fn make_fromgodot_for_gstring_enum(name: &Ident, enum_: &CStyleEnum) -> TokenStream {
    let names = enum_.names();
    let names_str = names.iter().map(ToString::to_string).collect::<Vec<_>>();
    let bad_variant_error = format!("invalid {name} variant");

    quote! {
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
    }
}
