/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Definition of engine enums/bitfields and related types.
//!
//! See also generator/enums.rs for functions related to turning enums into `TokenStream`s.

use crate::util::ident;

use proc_macro2::{Ident, Literal, TokenStream};
use quote::{quote, ToTokens};

pub struct Enum {
    pub name: Ident,
    pub godot_name: String,
    pub is_bitfield: bool,
    pub enumerators: Vec<Enumerator>,
}

impl Enum {
    /// The type we use to represent values of this enum.
    pub fn ord_type(&self) -> Ident {
        if self.is_bitfield {
            ident("u64")
        } else {
            ident("i32")
        }
    }

    /// Returns all unique enumerator ords, sorted.
    ///
    /// Returns `None` if `self` is a bitfield.
    pub fn unique_ords(&self) -> Option<Vec<i32>> {
        let mut unique_ords = self
            .enumerators
            .iter()
            .map(|enumerator| enumerator.value.as_enum())
            .collect::<Option<Vec<i32>>>()?;

        unique_ords.sort();
        unique_ords.dedup();

        Some(unique_ords)
    }

    /// Returns tokens representing the engine trait this enum should implement.
    pub fn engine_trait(&self) -> TokenStream {
        if self.is_bitfield {
            quote! { crate::obj::EngineBitfield }
        } else {
            quote! { crate::obj::EngineEnum }
        }
    }

    /// Returns the maximum index of an indexable enum.
    ///
    /// Return `None` if `self` isn't an indexable enum. Meaning it is either a bitfield, or it is an enum that can't be used as an index.
    pub fn count_index_enum(&self) -> Option<usize> {
        if self.is_bitfield {
            return None;
        }

        // Sort by ordinal value. Allocates for every enum in the JSON, but should be OK (most enums are indexable).
        let enumerators = {
            let mut enumerators = self.enumerators.clone();
            enumerators.sort_by_key(|v| v.value.to_i64());
            enumerators
        };

        // Highest ordinal must be the "MAX" one.
        // The presence of "MAX" indicates that Godot devs intended the enum to be used as an index.
        // The condition is not strictly necessary and could theoretically be relaxed; there would need to be concrete use cases though.
        let last = enumerators.last()?; // If there isn't a last we can assume it shouldn't be used as an index.

        if !last.godot_name.ends_with("_MAX") {
            return None;
        }

        // The rest of the enumerators must be contiguous and without gaps (duplicates are OK).
        let mut last_value = 0;

        for enumerator in enumerators.iter() {
            let current_value = enumerator.value.to_i64();

            if !(current_value == last_value || current_value == last_value + 1) {
                return None;
            }

            last_value = current_value;
        }

        Some(last_value as usize)
    }
}

#[derive(Clone)]
pub struct Enumerator {
    pub name: Ident,

    pub godot_name: String,

    // i64 is common denominator for enum, bitfield and constant values.
    // Note that values > i64::MAX will be implicitly wrapped, see https://github.com/not-fl3/nanoserde/issues/89.
    pub value: EnumeratorValue,
}

#[derive(Clone)]
pub enum EnumeratorValue {
    Enum(i32),
    Bitfield(u64),
}

impl EnumeratorValue {
    /// Tries to convert `self` into an enum value.
    fn as_enum(&self) -> Option<i32> {
        match self {
            EnumeratorValue::Enum(i) => Some(*i),
            EnumeratorValue::Bitfield(_) => None,
        }
    }

    /// Converts `self` to an `i64`.
    ///
    /// This may map some bitfield values to negative numbers.
    pub fn to_i64(&self) -> i64 {
        // Conversion is safe because i64 is used in the original JSON.
        match self {
            EnumeratorValue::Enum(i) => *i as i64,
            EnumeratorValue::Bitfield(i) => *i as i64,
        }
    }

    /// This method is needed for platform-dependent types like raw `VariantOperator`, which can be `i32` or `u32`.
    /// Do not suffix them.
    ///
    /// See also `BuiltinVariant::unsuffixed_ord_lit()`.
    pub fn unsuffixed_lit(&self) -> Literal {
        Literal::i64_unsuffixed(self.to_i64())
    }
}

impl ToTokens for EnumeratorValue {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            EnumeratorValue::Enum(i) => i.to_tokens(tokens),
            EnumeratorValue::Bitfield(i) => i.to_tokens(tokens),
        }
    }
}
