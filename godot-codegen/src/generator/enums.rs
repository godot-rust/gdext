/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::models::domain::{Enum, Enumerator, EnumeratorValue};
use crate::util;
use proc_macro2::{Literal, TokenStream};
use quote::quote;

pub fn make_enums(enums: &[Enum]) -> TokenStream {
    let definitions = enums.iter().map(make_enum_definition);

    quote! {
        #( #definitions )*
    }
}

pub fn make_enum_definition(enum_: &Enum) -> TokenStream {
    // TODO enums which have unique ords could be represented as Rust enums
    // This would allow exhaustive matches (or at least auto-completed matches + #[non_exhaustive]). But even without #[non_exhaustive],
    // this might be a forward compatibility hazard, if Godot deprecates enumerators and adds new ones with existing ords.

    let rust_enum_name = &enum_.name;
    let godot_name_doc = if rust_enum_name != enum_.godot_name.as_str() {
        let doc = format!("Godot enum name: `{}`.", enum_.godot_name);
        quote! { #[doc = #doc] }
    } else {
        TokenStream::new()
    };

    let rust_enumerators = &enum_.enumerators;

    let mut enumerators = Vec::with_capacity(rust_enumerators.len());

    // This is only used for enum ords (i32), not bitfield flags (u64).
    let mut unique_ords = Vec::with_capacity(rust_enumerators.len());

    for enumerator in rust_enumerators.iter() {
        let def = make_enumerator_definition(enumerator);
        enumerators.push(def);

        if let EnumeratorValue::Enum(ord) = enumerator.value {
            unique_ords.push(ord);
        }
    }

    let mut derives = vec!["Copy", "Clone", "Eq", "PartialEq", "Hash", "Debug"];

    if enum_.is_bitfield {
        derives.push("Default");
    }

    let derives = derives.into_iter().map(util::ident);

    let index_enum_impl = if enum_.is_bitfield {
        // Bitfields don't implement IndexEnum.
        TokenStream::new()
    } else {
        // Enums implement IndexEnum only if they are "index-like" (see docs).
        if let Some(enum_max) = try_count_index_enum(enum_) {
            quote! {
                impl crate::obj::IndexEnum for #rust_enum_name {
                    const ENUMERATOR_COUNT: usize = #enum_max;
                }
            }
        } else {
            TokenStream::new()
        }
    };

    let bitfield_ops;
    let self_as_trait;
    let engine_impl;
    let enum_ord_type;

    if enum_.is_bitfield {
        bitfield_ops = quote! {
            // impl #enum_name {
            //     pub const UNSET: Self = Self { ord: 0 };
            // }
            impl std::ops::BitOr for #rust_enum_name {
                type Output = Self;

                fn bitor(self, rhs: Self) -> Self::Output {
                    Self { ord: self.ord | rhs.ord }
                }
            }
        };
        enum_ord_type = quote! { u64 };
        self_as_trait = quote! { <Self as crate::obj::EngineBitfield> };
        engine_impl = quote! {
            impl crate::obj::EngineBitfield for #rust_enum_name {
                fn try_from_ord(ord: u64) -> Option<Self> {
                    Some(Self { ord })
                }

                fn ord(self) -> u64 {
                    self.ord
                }
            }
        };
    } else {
        // Ordinals are not necessarily in order.
        unique_ords.sort();
        unique_ords.dedup();

        bitfield_ops = TokenStream::new();
        enum_ord_type = quote! { i32 };
        self_as_trait = quote! { <Self as crate::obj::EngineEnum> };
        engine_impl = quote! {
            impl crate::obj::EngineEnum for #rust_enum_name {
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
        };
    };

    // Enumerator ordinal stored as i32, since that's enough to hold all current values and the default repr in C++.
    // Public interface is i64 though, for consistency (and possibly forward compatibility?).
    // Bitfield ordinals are stored as u64. See also: https://github.com/godotengine/godot-cpp/pull/1320
    quote! {
        #[repr(transparent)]
        #[derive(#( #derives ),*)]
        #godot_name_doc
        pub struct #rust_enum_name {
            ord: #enum_ord_type
        }
        impl #rust_enum_name {
            #(
                #enumerators
            )*
        }

        #engine_impl
        #index_enum_impl
        #bitfield_ops

        impl crate::builtin::meta::GodotConvert for #rust_enum_name {
            type Via = #enum_ord_type;
        }

        impl crate::builtin::meta::ToGodot for #rust_enum_name {
            fn to_godot(&self) -> Self::Via {
                #self_as_trait::ord(*self)
            }
        }

        impl crate::builtin::meta::FromGodot for #rust_enum_name {
            fn try_from_godot(via: Self::Via) -> std::result::Result<Self, crate::builtin::meta::ConvertError> {
                #self_as_trait::try_from_ord(via)
                    .ok_or_else(|| crate::builtin::meta::FromGodotError::InvalidEnum.into_error(via))
            }
        }
    }
}

pub fn make_enumerator_ord(ord: i32) -> Literal {
    Literal::i32_suffixed(ord)
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Implementation

fn make_bitfield_flag_ord(ord: u64) -> Literal {
    Literal::u64_suffixed(ord)
}

fn make_enumerator_definition(enumerator: &Enumerator) -> TokenStream {
    let ordinal_lit = match enumerator.value {
        EnumeratorValue::Enum(ord) => make_enumerator_ord(ord),
        EnumeratorValue::Bitfield(ord) => make_bitfield_flag_ord(ord),
    };

    let rust_ident = &enumerator.name;
    let godot_name_str = &enumerator.godot_name;

    let doc = if rust_ident == godot_name_str {
        TokenStream::new()
    } else {
        let doc_string = format!("Godot enumerator name: `{}`.", godot_name_str);
        quote! {
            #[doc(alias = #godot_name_str)]
            #[doc = #doc_string]
        }
    };

    quote! {
        #doc
        pub const #rust_ident: Self = Self { ord: #ordinal_lit };
    }
}

/// If an enum qualifies as "indexable" (can be used as array index), returns the number of possible values.
///
/// See `godot::obj::IndexEnum` for what constitutes "indexable".
fn try_count_index_enum(enum_: &Enum) -> Option<usize> {
    if enum_.is_bitfield || enum_.enumerators.is_empty() {
        return None;
    }

    // Sort by ordinal value. Allocates for every enum in the JSON, but should be OK (most enums are indexable).
    let enumerators = {
        let mut enumerators = enum_.enumerators.iter().collect::<Vec<_>>();
        enumerators.sort_by_key(|v| v.value.to_i64());
        enumerators
    };

    // Highest ordinal must be the "MAX" one.
    // The presence of "MAX" indicates that Godot devs intended the enum to be used as an index.
    // The condition is not strictly necessary and could theoretically be relaxed; there would need to be concrete use cases though.
    let last = enumerators.last().unwrap(); // safe because of is_empty check above.
    if !last.godot_name.ends_with("_MAX") {
        return None;
    }

    // The rest of the enumerators must be contiguous and without gaps (duplicates are OK).
    let mut last_value = 0;
    for enumerator in enumerators.iter() {
        let e_value = enumerator.value.to_i64();

        if last_value != e_value && last_value + 1 != e_value {
            return None;
        }

        last_value = e_value;
    }

    Some(last_value as usize)
}
