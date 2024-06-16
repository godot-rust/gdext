/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Built-in types like `Vector2`, `GString` and `Variant`.
//!
//! # Background on the design of vector algebra types
//!
//! The basic vector algebra types like `Vector2`, `Matrix4` and `Quaternion` are re-implemented
//! here, with an API similar to that in the Godot engine itself. There are other approaches, but
//! they all have their disadvantages:
//!
//! - We could invoke API methods from the engine. The implementations could be generated, but it
//!   is slower and prevents inlining.
//!
//! - We could re-export types from an existing vector algebra crate, like `glam`. This removes the
//!   duplication, but it would create a strong dependency on a volatile API outside our control.
//!   The `gdnative` crate started out this way, using types from `euclid`, but [found it
//!   impractical](https://github.com/godot-rust/gdnative/issues/594#issue-705061720). Moreover,
//!   the API would not match Godot's own, which would make porting from GDScript (slightly)
//!   harder.
//!
//! - We could opaquely wrap types from an existing vector algebra crate. This protects users of
//!   `gdextension` from changes in the wrapped crate. However, direct field access using `.x`,
//!   `.y`, `.z` is no longer possible. Instead of `v.y += a;` you would have to write
//!   `v.set_y(v.get_y() + a);`. (A `union` could be used to add these fields in the public API,
//!   but would make every field access unsafe, which is also not great.)
//!
//! - We could re-export types from the [`mint`](https://crates.io/crates/mint) crate, which was
//!   explicitly designed to solve this problem. However, it falls short because [operator
//!   overloading would become impossible](https://github.com/kvark/mint/issues/75).

// Re-export macros.
pub use crate::{array, dict, real, reals, varray};

// Re-export generated enums.
pub use crate::gen::central::global_reexported_enums::{Corner, EulerOrder, Side, VariantOperator};
pub use crate::sys::VariantType;
// Not yet public.
pub(crate) use crate::gen::central::VariantDispatch;

#[doc(hidden)]
pub mod __prelude_reexport {
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
    pub use string::{GString, NodePath, StringName};
    pub use transform2d::*;
    pub use transform3d::*;
    pub use variant::*;
    pub use vectors::*;

    pub use super::{EulerOrder, Side, VariantOperator, VariantType};
    pub use crate::{array, dict, real, reals, varray};
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
    pub use super::string::TransientStringNameOrd;
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

pub(crate) fn to_i64(i: usize) -> i64 {
    i.try_into().unwrap()
}

pub(crate) fn to_usize(i: i64) -> usize {
    i.try_into().unwrap()
}

pub(crate) fn to_isize(i: usize) -> isize {
    i.try_into().unwrap()
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Deprecated symbols

/// Specialized types related to arrays.
#[deprecated = "Merged into `godot::builtin::iter`."]
#[doc(hidden)] // No longer advertise in API docs.
pub mod array {
    pub type Iter<'a, T> = super::iter::ArrayIter<'a, T>;
}

/// Specialized types related to dictionaries.
#[deprecated = "Merged into `godot::builtin::iter`."]
#[doc(hidden)] // No longer advertise in API docs.
pub mod dictionary {
    pub type Iter<'a> = super::iter::DictIter<'a>;
    pub type Keys<'a> = super::iter::DictKeys<'a>;
    pub type TypedIter<'a, K, V> = super::iter::DictTypedIter<'a, K, V>;
    pub type TypedKeys<'a, K> = super::iter::DictTypedKeys<'a, K>;
}

#[deprecated = "Moved to `godot::meta` and submodules."]
#[doc(hidden)] // No longer advertise in API docs.
pub mod meta {
    pub use crate::meta::error::*;
    pub use crate::meta::*;
}

/// The side of a [`Rect2`] or [`Rect2i`].
///
/// _Godot equivalent: `@GlobalScope.Side`_
#[deprecated = "Merged with `godot::builtin::Side`."]
#[doc(hidden)] // No longer advertise in API docs.
pub type RectSide = Side;

#[allow(non_upper_case_globals)]
#[doc(hidden)] // No longer advertise in API docs.
impl Side {
    #[deprecated(note = "Renamed to `Side::LEFT`.")]
    pub const Left: Side = Side::LEFT;

    #[deprecated(note = "Renamed to `Side::TOP`.")]
    pub const Top: Side = Side::TOP;

    #[deprecated(note = "Renamed to `Side::RIGHT`.")]
    pub const Right: Side = Side::RIGHT;

    #[deprecated(note = "Renamed to `Side::BOTTOM`.")]
    pub const Bottom: Side = Side::BOTTOM;
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
