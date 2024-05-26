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

    pub use aabb::Aabb;
    pub use array_inner::{Array, VariantArray};
    pub use basis::Basis;
    pub use callable::{Callable, RustCallable};
    pub use color::{Color, ColorChannelOrder};
    pub use color_hsv::ColorHsv;
    pub use dictionary_inner::Dictionary;
    pub use packed_array::*;
    pub use plane::Plane;
    pub use projection::{Projection, ProjectionEye, ProjectionPlane};
    pub use quaternion::Quaternion;
    pub use real_inner::*;
    pub use rect2::Rect2;
    pub use rect2i::Rect2i;
    pub use rid::Rid;
    pub use signal::Signal;
    pub use string::{GString, NodePath, StringName};
    pub use transform2d::Transform2D;
    pub use transform3d::Transform3D;
    pub use variant::Variant;
    pub use vectors::{
        swizzle, ToVector, Vector2, Vector2Axis, Vector2i, Vector3, Vector3Axis, Vector3i,
        Vector4, Vector4Axis, Vector4i,
    };

    pub use super::{EulerOrder, Side, VariantOperator, VariantType};
    pub use crate::{array, dict, real, reals, varray};
}

pub use __prelude_reexport::*;

/// Meta-information about variant types, properties and class names.
pub mod meta;

/// Math-related functions and traits like [`ApproxEq`][math::ApproxEq].
pub mod math;

/// Specialized types related to arrays.
pub mod array {
    pub use super::array_inner::Iter;
}

/// Specialized types related to dictionaries.
pub mod dictionary {
    pub use super::dictionary_inner::{Iter, Keys, TypedIter, TypedKeys};
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
mod color;
mod color_constants; // After color, so that constants are listed after methods in docs (alphabetic ensures that).
mod color_hsv;
mod packed_array;
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
#[path = "array.rs"]
mod array_inner;
#[path = "dictionary.rs"]
mod dictionary_inner;
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

pub(crate) fn u8_to_bool(u: u8) -> bool {
    match u {
        0 => false,
        1 => true,
        _ => panic!("Invalid boolean value {u}"),
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Deprecated enums

/// The side of a [`Rect2`] or [`Rect2i`].
///
/// _Godot equivalent: `@GlobalScope.Side`_
#[deprecated = "Merged with `godot::builtin::Side`."]
pub type RectSide = Side;

#[allow(non_upper_case_globals)]
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
