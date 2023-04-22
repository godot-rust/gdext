/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Built-in types like `Vector2`, `GodotString` and `Variant`.
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
pub use crate::{array, dict, varray};

pub use aabb::*;
pub use array_inner::{Array, VariantArray};
pub use basis::*;
pub use callable::*;
pub use color::*;
pub use dictionary_inner::Dictionary;
pub use math::*;
pub use others::*;
pub use packed_array::*;
pub use plane::*;
pub use projection::*;
pub use quaternion::*;
pub use rect2::*;
pub use rect2i::*;
pub use rid::*;
pub use string::*;
pub use transform2d::*;
pub use transform3d::*;
pub use variant::*;
pub use vector2::*;
pub use vector2i::*;
pub use vector3::*;
pub use vector3i::*;
pub use vector4::*;
pub use vector4i::*;

/// Meta-information about variant types, properties and class names.
pub mod meta;

/// Specialized types related to arrays.
pub mod array {
    pub use super::array_inner::Iter;
}

/// Specialized types related to dictionaries.
pub mod dictionary {
    pub use super::dictionary_inner::{Iter, Keys, TypedIter, TypedKeys};
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Implementation

// Modules exporting declarative macros must appear first.
mod macros;
mod vector_macros;

// Rename imports because we re-export a subset of types under same module names.
#[path = "array.rs"]
mod array_inner;
#[path = "dictionary.rs"]
mod dictionary_inner;

mod aabb;
mod basis;
mod callable;
mod color;
mod glam_helpers;
mod math;
mod others;
mod packed_array;
mod plane;
mod projection;
mod quaternion;
mod rect2;
mod rect2i;
mod rid;
mod string;
mod transform2d;
mod transform3d;
mod variant;
mod vector2;
mod vector2i;
mod vector3;
mod vector3i;
mod vector4;
mod vector4i;

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

/// Clippy often complains if you do `f as f64` when `f` is already an `f64`. This trait exists to make it easy to
/// convert between the different reals and floats without a lot of allowing clippy lints for your code.
pub trait RealConv {
    /// Cast this [`real`][type@real] to an [`f32`] using `as`.
    // Clippy complains that this is an `as_*` function but it takes a `self`
    // however, since this uses `as` internally it makes much more sense for
    // it to be named `as_f32` rather than `to_f32`.
    #[allow(clippy::wrong_self_convention)]
    fn as_f32(self) -> f32;

    /// Cast this [`real`][type@real] to an [`f64`] using `as`.
    // Clippy complains that this is an `as_*` function but it takes a `self`
    // however, since this uses `as` internally it makes much more sense for
    // it to be named `as_f64` rather than `to_f64`.
    #[allow(clippy::wrong_self_convention)]
    fn as_f64(self) -> f64;

    /// Cast an [`f32`] to a [`real`][type@real] using `as`.
    fn from_f32(f: f32) -> Self;

    /// Cast an [`f64`] to a [`real`][type@real] using `as`.
    fn from_f64(f: f64) -> Self;
}

#[cfg(not(feature = "double-precision"))]
mod real_mod {
    /// Floating point type used for many structs and functions in Godot.
    ///
    /// This type is `f32` by default, and `f64` when the Cargo feature `double-precision` is enabled.
    ///
    /// This is not the `float` type in GDScript; that type is always 64-bits. Rather, many structs in Godot may use
    /// either 32-bit or 64-bit floats, for example [`Vector2`](super::Vector2). To convert between [`real`] and [`f32`] or
    /// [`f64`], see [`RealConv`](super::RealConv).
    ///
    /// See also the [Godot docs on float](https://docs.godotengine.org/en/stable/classes/class_float.html).
    ///
    /// _Godot equivalent: `real_t`_
    // As this is a scalar value, we will use a non-standard type name.
    #[allow(non_camel_case_types)]
    pub type real = f32;

    impl super::RealConv for real {
        #[inline]
        fn as_f32(self) -> f32 {
            self
        }

        #[inline]
        fn as_f64(self) -> f64 {
            self as f64
        }

        #[inline]
        fn from_f32(f: f32) -> Self {
            f
        }

        #[inline]
        fn from_f64(f: f64) -> Self {
            f as f32
        }
    }

    pub use std::f32::consts;

    /// A 2-dimensional vector from [`glam`]. Using a floating-point format compatible with [`real`].
    pub type RVec2 = glam::Vec2;
    /// A 3-dimensional vector from [`glam`]. Using a floating-point format compatible with [`real`].
    pub type RVec3 = glam::Vec3;
    /// A 4-dimensional vector from [`glam`]. Using a floating-point format compatible with [`real`].
    pub type RVec4 = glam::Vec4;

    /// A 2x2 column-major matrix from [`glam`]. Using a floating-point format compatible with [`real`].
    pub type RMat2 = glam::Mat2;
    /// A 3x3 column-major matrix from [`glam`]. Using a floating-point format compatible with [`real`].
    pub type RMat3 = glam::Mat3;
    /// A 4x4 column-major matrix from [`glam`]. Using a floating-point format compatible with [`real`].
    pub type RMat4 = glam::Mat4;

    /// A matrix from [`glam`] quaternion representing an orientation. Using a floating-point format compatible
    /// with [`real`].
    pub type RQuat = glam::Quat;

    /// A 2D affine transform from [`glam`], which can represent translation, rotation, scaling and
    /// shear. Using a floating-point format compatible with [`real`].
    pub type RAffine2 = glam::Affine2;
    /// A 3D affine transform from [`glam`], which can represent translation, rotation, scaling and
    /// shear. Using a floating-point format compatible with [`real`].
    pub type RAffine3 = glam::Affine3A;
}

#[cfg(feature = "double-precision")]
mod real_mod {
    /// Floating point type used for many structs and functions in Godot.
    ///
    /// This type is `f32` by default, and `f64` when the Cargo feature `double-precision` is enabled.
    ///
    /// This is not the `float` type in GDScript; that type is always 64-bits. Rather, many structs in Godot may use
    /// either 32-bit or 64-bit floats, for example [`Vector2`](super::Vector2). To convert between [`real`] and [`f32`] or
    /// [`f64`], see [`RealConv`](super::RealConv).
    ///
    /// See also the [Godot docs on float](https://docs.godotengine.org/en/stable/classes/class_float.html).
    ///
    /// _Godot equivalent: `real_t`_
    // As this is a scalar value, we will use a non-standard type name.
    #[allow(non_camel_case_types)]
    pub type real = f64;

    impl super::RealConv for real {
        #[inline]
        fn as_f32(self) -> f32 {
            self as f32
        }

        #[inline]
        fn as_f64(self) -> f64 {
            self
        }

        #[inline]
        fn from_f32(f: f32) -> Self {
            f as f64
        }

        #[inline]
        fn from_f64(f: f64) -> Self {
            f
        }
    }

    pub use std::f64::consts;

    /// A 2-dimensional vector from [`glam`]. Using a floating-point format compatible with [`real`].
    pub type RVec2 = glam::DVec2;
    /// A 3-dimensional vector from [`glam`]. Using a floating-point format compatible with [`real`].
    pub type RVec3 = glam::DVec3;
    /// A 4-dimensional vector from [`glam`]. Using a floating-point format compatible with [`real`].
    pub type RVec4 = glam::DVec4;

    /// A 2x2 column-major matrix from [`glam`]. Using a floating-point format compatible with [`real`].
    pub type RMat2 = glam::DMat2;
    /// A 3x3 column-major matrix from [`glam`]. Using a floating-point format compatible with [`real`].
    pub type RMat3 = glam::DMat3;
    /// A 4x4 column-major matrix from [`glam`]. Using a floating-point format compatible with [`real`].
    pub type RMat4 = glam::DMat4;

    /// A matrix from [`glam`] quaternion representing an orientation. Using a floating-point format
    /// compatible with [`real`].
    pub type RQuat = glam::DQuat;

    /// A 2D affine transform from [`glam`], which can represent translation, rotation, scaling and
    /// shear. Using a floating-point format compatible with [`real`].
    pub type RAffine2 = glam::DAffine2;
    /// A 3D affine transform from [`glam`], which can represent translation, rotation, scaling and
    /// shear. Using a floating-point format compatible with [`real`].
    pub type RAffine3 = glam::DAffine3;
}

pub use crate::real;
pub(crate) use real_mod::*;
pub use real_mod::{consts as real_consts, real};

pub(crate) use glam::{IVec2, IVec3, IVec4};

/// A macro to coerce float-literals into the real type. Mainly used where
/// you'd normally use a suffix to specity the type, such as `115.0f32`.
///
/// ### Examples
/// Rust will not know how to infer the type of this call to `to_radians`:
/// ```compile_fail
/// use godot_core::builtin::real;
///
/// let radians: real = 115.0.to_radians();
/// ```
/// But we can't add a suffix to the literal, since it may be either `f32` or
/// `f64` depending on the context. So instead we use our macro:
/// ```
/// use godot_core::builtin::real;
///
/// let radians: real = godot_core::real!(115.0).to_radians();
/// ```
#[macro_export]
macro_rules! real {
    ($f:literal) => {{
        let f: $crate::builtin::real = $f;
        f
    }};
}

/// The side of a [`Rect2`] or [`Rect2i`].
///
/// _Godot equivalent: `@GlobalScope.Side`_
#[derive(Copy, Clone)]
#[repr(C)]
pub enum RectSide {
    Left = 0,
    Top = 1,
    Right = 2,
    Bottom = 3,
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

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
