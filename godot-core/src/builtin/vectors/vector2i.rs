/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::cmp::Ordering;
use std::fmt;

use godot_ffi as sys;
use sys::{ffi_methods, ExtVariantType, GodotFfi};

use crate::builtin::math::{GlamConv, GlamType};
use crate::builtin::{inner, real, RVec2, Vector2, Vector2Axis};

/// Vector used for 2D math using integer coordinates.
///
/// 2-element structure that can be used to represent discrete positions or directions in 2D space,
/// as well as any other pair of numeric values.
///
/// `Vector2i` uses integer coordinates and is therefore preferable to [`Vector2`] when exact precision is
/// required. Note that the values are limited to 32 bits, and unlike `Vector2` this cannot be
/// configured with an engine build option. Use `i64` or [`PackedInt64Array`][crate::builtin::PackedInt64Array]
/// if 64-bit values are needed.
///
#[doc = shared_vector_docs!()]
///
/// ### Navigation to `impl` blocks within this page
///
/// - [Constants](#constants)
/// - [Constructors and general vector functions](#constructors-and-general-vector-functions)
/// - [Specialized `Vector2i` functions](#specialized-vector2i-functions)
/// - [2D functions](#2d-functions)
/// - [Trait impls + operators](#trait-implementations)
///
/// # All vector types
///
/// | Dimension | Floating-point                       | Integer                                |
/// |-----------|--------------------------------------|----------------------------------------|
/// | 2D        | [`Vector2`][crate::builtin::Vector2] | **`Vector2i`**                         |
/// | 3D        | [`Vector3`][crate::builtin::Vector3] | [`Vector3i`][crate::builtin::Vector3i] |
/// | 4D        | [`Vector4`][crate::builtin::Vector4] | [`Vector4i`][crate::builtin::Vector4i] |
///
/// <br>You can convert to `Vector2` using [`cast_float()`][Self::cast_float].
///
/// # Godot docs
///
/// [`Vector2i` (stable)](https://docs.godotengine.org/en/stable/classes/class_vector2i.html)
#[derive(Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(C)]
pub struct Vector2i {
    /// The vector's X component.
    pub x: i32,

    /// The vector's Y component.
    pub y: i32,
}

/// # Constants
impl Vector2i {
    impl_vector_consts!(i32);
    impl_integer_vector_consts!();
    impl_vector2x_consts!(i32);
}

/// # Specialized `Vector2i` functions
impl Vector2i {
    inline_impl_integer_vector_fns!(Vector2, x, y);

    /// Converts `self` to the corresponding [`real`] `glam` type.
    #[doc(hidden)]
    #[inline]
    pub fn to_glam_real(self) -> RVec2 {
        RVec2::new(self.x as real, self.y as real)
    }

    #[doc(hidden)]
    #[inline]
    pub fn as_inner(&self) -> inner::InnerVector2i<'_> {
        inner::InnerVector2i::from_outer(self)
    }
}

impl_vector_fns!(Vector2i, glam::IVec2, i32, (x, y));
impl_vector2x_fns!(Vector2i, Vector3i, i32);

impl_vector_operators!(Vector2i, i32, (x, y));

/// Formats the vector like Godot: `(x, y)`.
impl fmt::Display for Vector2i {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

// SAFETY:
// This type is represented as `Self` in Godot, so `*mut Self` is sound.
unsafe impl GodotFfi for Vector2i {
    const VARIANT_TYPE: ExtVariantType = ExtVariantType::Concrete(sys::VariantType::VECTOR2I);

    ffi_methods! { type sys::GDExtensionTypePtr = *mut Self; .. }
}

crate::meta::impl_godot_as_self!(Vector2i: ByValue);

impl GlamType for glam::IVec2 {
    type Mapped = Vector2i;

    fn to_front(&self) -> Self::Mapped {
        Vector2i::new(self.x, self.y)
    }

    fn from_front(mapped: &Self::Mapped) -> Self {
        glam::IVec2::new(mapped.x, mapped.y)
    }
}

impl GlamConv for Vector2i {
    type Glam = glam::IVec2;
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::assert_eq_approx;

    #[test]
    fn coord_min_max() {
        let a = Vector2i::new(1, 3);
        let b = Vector2i::new(0, 5);
        assert_eq!(a.coord_min(b), Vector2i::new(0, 3));
        assert_eq!(a.coord_max(b), Vector2i::new(1, 5));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_roundtrip() {
        let vector = Vector2i::default();
        let expected_json = "{\"x\":0,\"y\":0}";

        crate::builtin::test_utils::roundtrip(&vector, expected_json);
    }

    #[test]
    fn sign() {
        let vector = Vector2i::new(2, -5);
        assert_eq!(vector.sign(), Vector2i::new(1, -1));
        let vector = Vector2i::new(1, 0);
        assert_eq!(vector.sign(), Vector2i::new(1, 0));
    }

    #[test]
    fn axis_min_max() {
        assert_eq!(Vector2i::new(10, 5).max_axis(), Some(Vector2Axis::X));
        assert_eq!(Vector2i::new(5, 10).max_axis(), Some(Vector2Axis::Y));

        assert_eq!(Vector2i::new(-5, 5).min_axis(), Some(Vector2Axis::X));
        assert_eq!(Vector2i::new(5, -5).min_axis(), Some(Vector2Axis::Y));

        assert_eq!(Vector2i::new(15, 15).max_axis(), None);
        assert_eq!(Vector2i::new(15, 15).min_axis(), None);
    }

    #[test]
    fn distance() {
        let a = Vector2i::new(1, 2);
        let b = Vector2i::new(4, 6);

        assert_eq!(a.distance_squared_to(b), 25);
        assert_eq_approx!(a.distance_to(b), 5.0);
    }

    #[test]
    fn mini_maxi_clampi() {
        let v = Vector2i::new(10, -5);

        assert_eq!(v.mini(3), Vector2i::new(3, -5));
        assert_eq!(v.maxi(-2), Vector2i::new(10, -2));
        assert_eq!(v.clampi(-3, 7), Vector2i::new(7, -3));
    }

    #[test]
    fn snappedi() {
        let v = Vector2i::new(13, -8);

        assert_eq!(v.snappedi(5), Vector2i::new(15, -10));
    }
}
