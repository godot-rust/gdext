/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Built-in types like `Vector2`, `GString` and `Variant`.
//!
//! Please read the [book chapter](https://godot-rust.github.io/book/godot-api/builtins.html) about builtin types.
//!
//! # API design
//! API design behind the builtin types (and some wider parts of the library) is elaborated in the
//! [extended documentation page](../__docs/index.html#builtin-api-design).

use std::fmt::Debug;
use std::ops;

// Re-export generated enums.
pub use crate::gen::central::global_reexported_enums::{Corner, EulerOrder, Side, VariantOperator};
// Not yet public.
pub(crate) use crate::gen::central::VariantDispatch;
pub use crate::sys::VariantType;
// Re-export macros.
#[allow(deprecated)] // dict
pub use crate::{array, dict, real, reals, varray, vdict};

#[doc(hidden)]
pub mod __prelude_reexport {
    #[rustfmt::skip] // Do not reorder.
    use super::*;

    pub use aabb::*;
    pub use basis::*;
    pub use callable::*;
    pub use collections::containers::*;
    pub use color::*;
    pub use color_hsv::*;
    pub use plane::*;
    pub use projection::*;
    pub use quaternion::*;
    pub use real_inner::*;
    pub use rect2::*;
    pub use rect2i::*;
    pub use rid::*;
    pub use signal::*;
    pub use string::{Encoding, GString, NodePath, StringName};
    pub use transform2d::*;
    pub use transform3d::*;
    pub use variant::*;
    pub use vectors::*;

    pub use super::math::XformInv;
    pub use super::{EulerOrder, Side, VariantOperator, VariantType};
    pub use crate::{array, real, reals, varray, vdict, vslice};

    #[allow(deprecated)]
    #[rustfmt::skip] // Do not reorder.
    pub use crate::dict;
}

pub use __prelude_reexport::*;

/// Math-related functions and traits like [`ApproxEq`][math::ApproxEq].
pub mod math;

/// Iterator types for arrays and dictionaries.
pub mod iter {
    pub use super::collections::iterators::*;
}

/// Specialized types related to Godot's various string implementations.
pub mod strings {
    pub use super::string::{
        ExGStringFind, ExGStringSplit, ExStringNameFind, ExStringNameSplit, TransientStringNameOrd,
    };
}

pub(crate) mod meta_reexport {
    pub use super::collections::PackedArrayElement;
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Implementation

// Modules exporting declarative macros must appear first.
mod macros;

// Other modules
mod aabb;
mod basis;
mod callable;
mod collections;
mod color;
mod color_constants; // After color, so that constants are listed after methods in docs (alphabetic ensures that).
mod color_hsv;
mod plane;
mod projection;
mod quaternion;
mod rect2;
mod rect2i;
mod rid;
mod signal;
mod string;
mod transform2d;
mod transform3d;
mod variant;
mod vectors;

// Rename imports because we re-export a subset of types under same module names.
#[path = "real.rs"]
mod real_inner;

#[doc(hidden)]
pub mod inner {
    pub use crate::gen::builtin_classes::*;
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// utils

pub(crate) fn to_i64(i: usize) -> i64 {
    i.try_into().unwrap()
}

pub(crate) fn to_usize(i: i64) -> usize {
    i.try_into().unwrap()
}

/// Trait easing converting Rust ranges to values expected by Godot.
///
/// Note: Unbounded upper bounds must be represented by `i32::MAX` instead of `i64::MAX`,
/// since Godot treats some indexes as 32-bit despite being declared `i64` in GDExtension API.
pub(crate) trait GodotRange<T> {
    /// Returns a tuple of `(from, to)` from a Rust range.
    fn to_godot_range_fromto(&self) -> (i64, Option<i64>);

    /// Returns a tuple of `(from, len)` from a Rust range.
    ///
    /// Unbounded upper bound will be represented by `from = default_unbounded_upper`.
    ///
    /// # Panics
    /// In debug mode, when `from` > `to`.
    fn to_godot_range_fromto_checked(&self, default_unbounded_upper: i64) -> (i64, i64) {
        match self.to_godot_range_fromto() {
            (from, Some(to)) => {
                debug_assert!(from <= to, "range: start ({from}) > end ({to})");
                (from, to)
            }
            (from, None) => (from, default_unbounded_upper),
        }
    }

    /// Returns a tuple of `(from, len)` from a Rust range.
    ///
    /// Unbounded upper bounds are represented by `len = default_unbounded_upper`.
    fn to_godot_range_fromlen(&self, default_unbounded_upper: i64) -> (i64, i64) {
        match self.to_godot_range_fromto() {
            (from, Some(to)) => {
                debug_assert!(from <= to, "range: start ({from}) > end ({to})");
                (from, to - from)
            }
            (from, None) => (from, default_unbounded_upper),
        }
    }
}

impl<T, R> GodotRange<T> for R
where
    R: ops::RangeBounds<T>,
    i64: TryFrom<T>,
    T: Copy + std::fmt::Display,
    <T as TryInto<i64>>::Error: Debug,
{
    fn to_godot_range_fromto(&self) -> (i64, Option<i64>) {
        let from = match self.start_bound() {
            ops::Bound::Included(&n) => i64::try_from(n).unwrap(),
            ops::Bound::Excluded(&n) => i64::try_from(n).unwrap() + 1,
            ops::Bound::Unbounded => 0,
        };

        let to = match self.end_bound() {
            ops::Bound::Included(&n) => {
                let to = i64::try_from(n).unwrap() + 1;
                Some(to)
            }
            ops::Bound::Excluded(&n) => {
                let to = i64::try_from(n).unwrap();
                Some(to)
            }
            ops::Bound::Unbounded => None,
        };

        (from, to)
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// #[test] utils for serde

#[cfg(all(test, feature = "serde"))]
pub(crate) mod test_utils {
    use serde::{Deserialize, Serialize};

    pub(crate) fn roundtrip<T>(value: &T, expected_json: &str)
    where
        T: for<'a> Deserialize<'a> + Serialize + PartialEq + std::fmt::Debug,
    {
        let json: String = serde_json::to_string(value).unwrap();
        let back: T = serde_json::from_str(json.as_str()).unwrap();

        assert_eq!(back, *value, "serde round-trip changes value");
        assert_eq!(
            json, expected_json,
            "value does not conform to expected JSON"
        );
    }
}
