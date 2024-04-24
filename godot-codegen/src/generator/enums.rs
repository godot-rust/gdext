/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Functions for codegenning enums.
//!
//! See also models/domain/enums.rs for other enum-related methods.

use crate::{
    models::domain::{self, Enum},
    util::ident,
};
use proc_macro2::{Ident, TokenStream};
use quote::{quote, ToTokens};

pub fn make_enums(enums: &[domain::Enum]) -> TokenStream {
    let definitions = enums.iter().map(Enum::to_declaration);

    quote! {
        #( #definitions )*
    }
}

/// Codegen methods.
impl domain::Enum {
    /// Creates a declaration of this enum.
    ///
    /// This will also implement all relevant traits and generate appropriate constants for each enumerator.
    pub fn to_declaration(&self) -> TokenStream {
        // Things needed for the type definition
        let derives = self.derives();
        let enum_doc = self.enum_doc();
        let name = &self.name;

        // Values
        let enumerators = self.to_const_declarations();

        // Trait implementations
        let engine_trait_impl = self.to_engine_trait_impl();
        let index_enum_impl = self.to_index_impl();
        let bitwise_impls = self.bitwise_operators();

        // Various types
        let ord_type = self.ord_type();
        let engine_trait = self.engine_trait();

        quote! {
            #[repr(transparent)]
            #[derive(#( #derives ),*)]
            #( #[doc = #enum_doc] )*
            pub struct #name {
                ord: #ord_type
            }

            impl #name {
                #enumerators
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

    /// Creates an implementation of `IndexEnum` for this enum.
    ///
    /// Returns `None` if `self` isn't an indexable enum.
    fn to_index_impl(&self) -> Option<TokenStream> {
        let enum_max = self.count_index_enum()?;
        let name = &self.name;

        Some(quote! {
            impl crate::obj::IndexEnum for #name {
                const ENUMERATOR_COUNT: usize = #enum_max;
            }
        })
    }

    /// Creates an implementation of the engine trait for this enum.
    ///
    /// This will implement the trait returned by [`Enum::engine_trait`].
    fn to_engine_trait_impl(&self) -> TokenStream {
        let name = &self.name;
        let engine_trait = self.engine_trait();

        if self.is_bitfield {
            quote! {
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
            let unique_ords = self.unique_ords().expect("self is an enum");

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

    /// Creates declarations for all the constants of this enum.
    fn to_const_declarations(&self) -> TokenStream {
        let declarations = self
            .enumerators
            .iter()
            .map(|field| field.to_const_declaration((&self.name).into_token_stream()))
            .collect::<Vec<_>>();

        quote! {
            #( #declarations )*
        }
    }

    /// Creates implementations for any bitwise operators.
    ///
    /// Currently this is just [`BitOr`](std::ops::BitOr) for bitfields but that could be expanded in the future.
    fn bitwise_operators(&self) -> TokenStream {
        let name = &self.name;

        if self.is_bitfield {
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

    /// Which derives should be implemented for this enum.
    fn derives(&self) -> Vec<Ident> {
        let mut derives = vec!["Copy", "Clone", "Eq", "PartialEq", "Hash", "Debug"];

        if self.is_bitfield {
            derives.push("Default");
        }

        derives.into_iter().map(ident).collect()
    }

    /// Returns the documentation for this enum.
    ///
    /// Each string is one line of documentation, usually this needs to be wrapped in a `#[doc = ..]`.
    fn enum_doc(&self) -> Vec<String> {
        let mut docs = Vec::new();

        if self.name != self.godot_name {
            docs.push(format!("Godot enum name: `{}`.", self.godot_name))
        }

        docs
    }
}

/// Codegen methods.
impl domain::Enumerator {
    /// Creates a `const` declaration for self of the type `enum_type`.
    ///
    /// That is, it'll be a declaration like
    /// ```ignore
    /// pub const NAME: enum_type = ..;
    /// ```
    fn to_const_declaration(&self, enum_type: TokenStream) -> TokenStream {
        let Self {
            name,
            godot_name,
            value,
        } = self;

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
}
