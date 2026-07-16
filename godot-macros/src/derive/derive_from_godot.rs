/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use proc_macro2::{Ident, TokenStream};
use quote::quote;

use crate::derive::data_models::{CStyleEnum, ConvertType, GodotConvert, NewtypeStruct, ViaType};
use crate::derive::derive_godot_convert::EnumeratorExprCache;
use crate::util;

/// Creates a `FromGodot` impl for the given `GodotConvert`.
///
/// There is no dedicated `FromGodot` derive macro currently, this is instead called by the `GodotConvert` derive macro.
pub fn make_fromgodot(convert: &GodotConvert, cache: &mut EnumeratorExprCache) -> TokenStream {
    let GodotConvert {
        ty_name: name,
        convert_type: data,
        ..
    } = convert;

    match data {
        ConvertType::NewType { field } => make_fromgodot_for_newtype_struct(convert, field),

        ConvertType::Enum {
            variants,
            via: ViaType::GString { .. },
        } => make_fromgodot_for_gstring_enum(name, variants),

        ConvertType::Enum {
            variants,
            via: ViaType::Int { int_ident },
        } => make_fromgodot_for_int_enum(name, variants, int_ident, cache),
    }
}

/// Derives `FromGodot` for newtype structs.
fn make_fromgodot_for_newtype_struct(convert: &GodotConvert, field: &NewtypeStruct) -> TokenStream {
    // For tuple structs this ends up using the alternate tuple-struct constructor syntax of
    // TupleStruct { 0: value }
    let GodotConvert {
        ty_name: name,
        generic_params,
        where_clause,
        ..
    } = convert;

    let generic_args = generic_params
        .as_ref()
        .map(|params| params.as_inline_args());

    let field_name = &field.sized.ident;
    let via_type = &field.sized.ty;

    let field_zst_names = field.zsts.iter().map(|field| &field.ident);
    let field_zst_tys = field.zsts.iter().map(|field| &field.ty);

    // This is basically copy-paste of the unstable feature for creating arbitrary ZSTs.
    // https://github.com/rust-lang/rust/issues/95383
    let create_zst = quote! {
        const {
            #(assert!(size_of::<#field_zst_tys>() == 0, "Type is not a ZST");)*
            // SAFETY: because the caller must guarantee that it's inhabited and zero-sized,
            // there's nothing in the representation that needs to be set.
            // `assume_init` calls `assert_inhabited`, so we don't need to here.
            unsafe {
                // #[allow(clippy::uninit_assumed_init)]
                ::std::mem::MaybeUninit::uninit().assume_init()
            }
        }
    };

    quote! {
        impl #generic_params ::godot::meta::FromGodot for #name #generic_args #where_clause {
            fn try_from_godot(via: #via_type) -> ::std::result::Result<Self, ::godot::meta::error::ConvertError> {
                Ok(Self {
                    #field_name: via,
                    #(#field_zst_names: #create_zst),*
                })
            }
        }
    }
}

/// Derives `FromGodot` for enums with a via type of integers.
fn make_fromgodot_for_int_enum(
    name: &Ident,
    enum_: &CStyleEnum,
    int: &Ident,
    cache: &mut EnumeratorExprCache,
) -> TokenStream {
    let discriminants =
        cache.map_ord_exprs(int, enum_.enumerator_names(), enum_.enumerator_ord_exprs());
    let names = enum_.enumerator_names();
    let bad_variant_error = format!("invalid {name} variant");

    let ord_variables: Vec<Ident> = names
        .iter()
        .map(|e| util::ident(&format!("ORD_{e}")))
        .collect();

    quote! {
        impl ::godot::meta::FromGodot for #name {
            #[allow(unused_parens)] // Error "unnecessary parentheses around match arm expression"; comes from ord° expressions like (1 + 2).
            fn try_from_godot(via: #int) -> ::std::result::Result<Self, ::godot::meta::error::ConvertError> {
                #(
                    // Interesting: using let instead of const would introduce a runtime bug. Its values cannot be used in match lhs (binding).
                    // However, bindings silently shadow variables, so the first match arm always runs; no warning in generated proc-macro code.
                    #[allow(non_upper_case_globals)]
                    const #ord_variables: #int = #discriminants;
                )*

                match via {
                    #(
                        #ord_variables => Ok(#name::#names),
                    )*
                    // Pass `via` and not `other`, to retain debug info of original type.
                    other => Err(::godot::meta::error::ConvertError::with_error_value(#bad_variant_error, via))
                }
            }
        }
    }
}

/// Derives `FromGodot` for enums with a via type of `GString`.
fn make_fromgodot_for_gstring_enum(name: &Ident, enum_: &CStyleEnum) -> TokenStream {
    let names = enum_.enumerator_names();
    let names_str = names.iter().map(ToString::to_string).collect::<Vec<_>>();
    let bad_variant_error = format!("invalid {name} variant");

    quote! {
        impl ::godot::meta::FromGodot for #name {
            fn try_from_godot(via: ::godot::builtin::GString) -> ::std::result::Result<Self, ::godot::meta::error::ConvertError> {
                match via.to_string().as_str() {
                    #(
                        #names_str => Ok(#name::#names),
                    )*
                    // Pass `via` and not `other`, to retain debug info of original type.
                    other => Err(::godot::meta::error::ConvertError::with_error_value(#bad_variant_error, via))
                }
            }
        }
    }
}
