/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Functions for generating engine-provided enums.
//!
//! See also models/domain/enums.rs for other enum-related methods.

use crate::models::domain::{Enum, Enumerator};
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};

pub fn make_enums(enums: &[Enum]) -> TokenStream {
    let definitions = enums.iter().map(make_enum_definition);

    quote! {
        #( #definitions )*
    }
}

/// Creates a definition for the given enum.
///
/// This will also implement all relevant traits and generate appropriate constants for each enumerator.
pub fn make_enum_definition(enum_: &Enum) -> TokenStream {
    // Things needed for the type definition
    let derives = enum_.derives();
    let enum_doc = make_enum_doc(enum_);
    let name = &enum_.name;

    // Values
    let enumerators = enum_
        .enumerators
        .iter()
        .map(|enumerator| make_enumerator_definition(enumerator, name.to_token_stream()));

    // Trait implementations
    let engine_trait_impl = make_enum_engine_trait_impl(enum_);
    let index_enum_impl = make_enum_index_impl(enum_);
    let bitwise_impls = make_enum_bitwise_operators(enum_);

    // Various types
    let ord_type = enum_.ord_type();
    let engine_trait = enum_.engine_trait();

    quote! {
        #[repr(transparent)]
        #[derive( #( #derives ),* )]
        #( #[doc = #enum_doc] )*
        pub struct #name {
            ord: #ord_type
        }

        impl #name {
            #( #enumerators )*
        }

        #engine_trait_impl
        #index_enum_impl
        #bitwise_impls

        impl crate::builtin::meta::GodotConvert for #name {
            type Via = #ord_type;
        }

        impl crate::builtin::meta::ToGodot for #name {
            fn to_godot(&self) -> Self::Via {
                <Self as #engine_trait>::ord(*self)
            }
        }

        impl crate::builtin::meta::FromGodot for #name {
            fn try_from_godot(via: Self::Via) -> std::result::Result<Self, crate::builtin::meta::ConvertError> {
                <Self as #engine_trait>::try_from_ord(via)
                    .ok_or_else(|| crate::builtin::meta::FromGodotError::InvalidEnum.into_error(via))
            }
        }
    }
}

/// Creates an implementation of `IndexEnum` for the given enum.
///
/// Returns `None` if `enum_` isn't an indexable enum.
fn make_enum_index_impl(enum_: &Enum) -> Option<TokenStream> {
    let enum_max = enum_.find_index_enum_max()?;
    let name = &enum_.name;

    Some(quote! {
        impl crate::obj::IndexEnum for #name {
            const ENUMERATOR_COUNT: usize = #enum_max;
        }
    })
}

/// Creates an implementation of the engine trait for the given enum.
///
/// This will implement the trait returned by [`Enum::engine_trait`].
fn make_enum_engine_trait_impl(enum_: &Enum) -> TokenStream {
    let name = &enum_.name;
    let engine_trait = enum_.engine_trait();

    if enum_.is_bitfield {
        quote! {
            // We may want to add this in the future.
            //
            // impl #enum_name {
            //     pub const UNSET: Self = Self { ord: 0 };
            // }

            impl #engine_trait for #name {
                fn try_from_ord(ord: u64) -> Option<Self> {
                    Some(Self { ord })
                }

                fn ord(self) -> u64 {
                    self.ord
                }
            }
        }
    } else {
        let unique_ords = enum_.unique_ords().expect("self is an enum");

        quote! {
            impl #engine_trait for #name {
                fn try_from_ord(ord: i32) -> Option<Self> {
                    match ord {
                        #( ord @ #unique_ords )|* => Some(Self { ord }),
                        _ => None,
                    }
                }

                fn ord(self) -> i32 {
                    self.ord
                }
            }
        }
    }
}

/// Creates implementations for bitwise operators for the given enum.
///
/// Currently this is just [`BitOr`](std::ops::BitOr) for bitfields but that could be expanded in the future.
fn make_enum_bitwise_operators(enum_: &Enum) -> TokenStream {
    let name = &enum_.name;

    if enum_.is_bitfield {
        quote! {
            impl std::ops::BitOr for #name {
                type Output = Self;

                fn bitor(self, rhs: Self) -> Self::Output {
                    Self { ord: self.ord | rhs.ord }
                }
            }
        }
    } else {
        TokenStream::new()
    }
}
/// Returns the documentation for the given enum.
///
/// Each string is one line of documentation, usually this needs to be wrapped in a `#[doc = ..]`.
fn make_enum_doc(enum_: &Enum) -> Vec<String> {
    let mut docs = Vec::new();

    if enum_.name != enum_.godot_name {
        docs.push(format!("Godot enum name: `{}`.", enum_.godot_name))
    }

    docs
}

/// Creates a `const` definition for `enumerator` of the type `enum_type`.
///
/// That is, it'll be a definition like
/// ```ignore
/// pub const NAME: enum_type = ..;
/// ```
fn make_enumerator_definition(enumerator: &Enumerator, enum_type: TokenStream) -> TokenStream {
    let Enumerator {
        name,
        godot_name,
        value,
    } = enumerator;

    let docs = if &name.to_string() != godot_name {
        let doc = format!("Godot enumerator name: `{godot_name}`");

        quote! {
            #[doc(alias = #godot_name)]
            #[doc = #doc]
        }
    } else {
        TokenStream::new()
    };

    quote! {
        #docs
        pub const #name: #enum_type = #enum_type {
            ord: #value
        };
    }
}
