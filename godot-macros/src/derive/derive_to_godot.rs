/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use proc_macro2::{Ident, TokenStream};
use quote::quote;

use crate::derive::data_models::{CStyleEnum, ConvertType, GodotConvert, NewtypeStruct, ViaType};

/// Creates a `ToGodot` impl for the given `GodotConvert`.
///
/// There is no dedicated `ToGodot` derive macro currently, this is instead called by the `GodotConvert` derive macro.
pub fn make_togodot(convert: &GodotConvert) -> TokenStream {
    let GodotConvert {
        ty_name: name,
        convert_type: data,
    } = convert;

    match data {
        ConvertType::NewType { field } => make_togodot_for_newtype_struct(name, field),
        ConvertType::Enum {
            variants,
            via: ViaType::GString { .. },
        } => make_togodot_for_string_enum(name, variants),
        ConvertType::Enum {
            variants,
            via: ViaType::Int { int_ident },
        } => make_togodot_for_int_enum(name, variants, int_ident),
    }
}

/// Derives `ToGodot` for newtype structs.
fn make_togodot_for_newtype_struct(name: &Ident, field: &NewtypeStruct) -> TokenStream {
    let field_name = field.field_name();
    let via_type = &field.ty;

    quote! {
        impl ::godot::builtin::meta::ToGodot for #name {
            fn to_godot(&self) -> #via_type {
                ::godot::builtin::meta::ToGodot::to_godot(&self.#field_name)
            }

            fn into_godot(self) -> #via_type {
                ::godot::builtin::meta::ToGodot::into_godot(self.#field_name)
            }
        }
    }
}

/// Derives `ToGodot` for enums with a via type of integers.
fn make_togodot_for_int_enum(name: &Ident, enum_: &CStyleEnum, int: &Ident) -> TokenStream {
    let discriminants = enum_.discriminants();
    let names = enum_.names();

    quote! {
        impl ::godot::builtin::meta::ToGodot for #name {
            fn to_godot(&self) -> #int {
                match self {
                    #(
                        #name::#names => #discriminants,
                    )*
                }
            }
        }
    }
}

/// Derives `ToGodot` for enums with a via type of `GString`.
fn make_togodot_for_string_enum(name: &Ident, enum_: &CStyleEnum) -> TokenStream {
    let names = enum_.names();
    let names_str = names.iter().map(ToString::to_string).collect::<Vec<_>>();

    quote! {
        impl ::godot::builtin::meta::ToGodot for #name {
            fn to_godot(&self) -> ::godot::builtin::GString {
                match self {
                    #(
                        #name::#names => #names_str.into(),
                    )*
                }
            }
        }
    }
}
