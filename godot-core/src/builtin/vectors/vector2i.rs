/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot_ffi as sys;
use std::cmp::Ordering;
use sys::{ffi_methods, GodotFfi};

use crate::builtin::math::{GlamConv, GlamType};
use crate::builtin::{inner, real, RVec2, Vector2, Vector2Axis};

use std::fmt;

/// Vector used for 2D math using integer coordinates.
///
/// 2-element structure that can be used to represent discrete positions or directions in 2D space,
/// as well as any other pair of numeric values.
///
/// It uses integer coordinates and is therefore preferable to [`Vector2`] when exact precision is
/// required. Note that the values are limited to 32 bits, and unlike `Vector2` this cannot be
/// configured with an engine build option. Use `i64` or [`PackedInt64Array`][crate::builtin::PackedInt64Array]
/// if 64-bit values are needed.
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
    inline_impl_integer_vector_fns!(x, y);

    /// Constructs a new `Vector2i` from a [`Vector2`]. The floating point coordinates will be truncated.
    #[inline]
    pub const fn from_vector2(v: Vector2) -> Self {
        Self {
            x: v.x as i32,
            y: v.y as i32,
        }
    }

    /// Converts `self` to the corresponding [`real`] `glam` type.
    #[doc(hidden)]
    #[inline]
    pub fn to_glam_real(self) -> RVec2 {
        RVec2::new(self.x as real, self.y as real)
    }

    #[doc(hidden)]
    #[inline]
    pub fn as_inner(&self) -> inner::InnerVector2i {
        inner::InnerVector2i::from_outer(self)
    }
}

impl_vector_fns!(Vector2i, glam::IVec2, i32, (x, y));
impl_vector2x_fns!(Vector2i, i32);

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
    fn variant_type() -> sys::VariantType {
        sys::VariantType::VECTOR2I
    }

    ffi_methods! { type sys::GDExtensionTypePtr = *mut Self; .. }
}

crate::meta::impl_godot_as_self!(Vector2i);

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
    fn axis_min_max() {
        assert_eq!(Vector2i::new(10, 5).max_axis(), Some(Vector2Axis::X));
        assert_eq!(Vector2i::new(5, 10).max_axis(), Some(Vector2Axis::Y));

        assert_eq!(Vector2i::new(-5, 5).min_axis(), Some(Vector2Axis::X));
        assert_eq!(Vector2i::new(5, -5).min_axis(), Some(Vector2Axis::Y));

        assert_eq!(Vector2i::new(15, 15).max_axis(), None);
        assert_eq!(Vector2i::new(15, 15).min_axis(), None);
    }
}
