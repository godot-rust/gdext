/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::cmp::Ordering;
use std::fmt;

use godot_ffi as sys;
use sys::{ffi_methods, GodotFfi};

use crate::builtin::math::{GlamConv, GlamType};
use crate::builtin::{inner, real, RVec3, Vector3, Vector3Axis};

/// Vector used for 3D math using integer coordinates.
///
/// 3-element structure that can be used to represent discrete positions or directions in 3D space,
/// as well as any other triple of numeric values.
///
/// It uses integer coordinates and is therefore preferable to [`Vector3`] when exact precision is
/// required. Note that the values are limited to 32 bits, and unlike `Vector3` this cannot be
/// configured with an engine build option. Use `i64` or [`PackedInt64Array`][crate::builtin::PackedInt64Array]
/// if 64-bit values are needed.
///
/// ### Navigation to `impl` blocks within this page
///
/// - [Constants](#constants)
/// - [Constructors and general vector functions](#constructors-and-general-vector-functions)
/// - [Specialized `Vector3i` functions](#specialized-vector3i-functions)
/// - [3D functions](#3d-functions)
/// - [Trait impls + operators](#trait-implementations)
///
/// # All vector types
///
/// | Dimension | Floating-point                       | Integer                                |
/// |-----------|--------------------------------------|----------------------------------------|
/// | 2D        | [`Vector2`][crate::builtin::Vector2] | [`Vector2i`][crate::builtin::Vector2i] |
/// | 3D        | [`Vector3`][crate::builtin::Vector3] | **`Vector3i`**                         |
/// | 4D        | [`Vector4`][crate::builtin::Vector4] | [`Vector4i`][crate::builtin::Vector4i] |
#[derive(Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(C)]
pub struct Vector3i {
    /// The vector's X component.
    pub x: i32,

    /// The vector's Y component.
    pub y: i32,

    /// The vector's Z component.
    pub z: i32,
}

/// # Constants
impl Vector3i {
    impl_vector_consts!(i32);
    impl_integer_vector_consts!();
    impl_vector3x_consts!(i32);
}

impl_vector_fns!(Vector3i, glam::IVec3, i32, (x, y, z));

/// # Specialized `Vector3i` functions
impl Vector3i {
    /// Constructs a new `Vector3i` from a [`Vector3`]. The floating point coordinates will be truncated.
    #[inline]
    pub const fn from_vector3(v: Vector3) -> Self {
        Self {
            x: v.x as i32,
            y: v.y as i32,
            z: v.z as i32,
        }
    }

    inline_impl_integer_vector_fns!(x, y, z);

    /// Converts `self` to the corresponding [`real`] `glam` type.
    #[doc(hidden)]
    #[inline]
    pub fn to_glam_real(self) -> RVec3 {
        RVec3::new(self.x as real, self.y as real, self.z as real)
    }

    #[doc(hidden)]
    #[inline]
    pub fn as_inner(&self) -> inner::InnerVector3i {
        inner::InnerVector3i::from_outer(self)
    }
}

impl_vector3x_fns!(Vector3i, i32);

impl_vector_operators!(Vector3i, i32, (x, y, z));

/// Formats the vector like Godot: `(x, y, z)`.
impl fmt::Display for Vector3i {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {}, {})", self.x, self.y, self.z)
    }
}

// SAFETY:
// This type is represented as `Self` in Godot, so `*mut Self` is sound.
unsafe impl GodotFfi for Vector3i {
    fn variant_type() -> sys::VariantType {
        sys::VariantType::VECTOR3I
    }

    ffi_methods! { type sys::GDExtensionTypePtr = *mut Self; .. }
}

crate::meta::impl_godot_as_self!(Vector3i);

impl GlamType for glam::IVec3 {
    type Mapped = Vector3i;

    fn to_front(&self) -> Self::Mapped {
        Vector3i::new(self.x, self.y, self.z)
    }

    fn from_front(mapped: &Self::Mapped) -> Self {
        glam::IVec3::new(mapped.x, mapped.y, mapped.z)
    }
}

impl GlamConv for Vector3i {
    type Glam = glam::IVec3;
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn coord_min_max() {
        let a = Vector3i::new(1, 3, 5);
        let b = Vector3i::new(0, 5, 2);
        assert_eq!(a.coord_min(b), Vector3i::new(0, 3, 2));
        assert_eq!(a.coord_max(b), Vector3i::new(1, 5, 5));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_roundtrip() {
        let vector = Vector3i::default();
        let expected_json = "{\"x\":0,\"y\":0,\"z\":0}";

        crate::builtin::test_utils::roundtrip(&vector, expected_json);
    }

    #[test]
    fn axis_min_max() {
        assert_eq!(Vector3i::new(10, 5, -5).max_axis(), Some(Vector3Axis::X));
        assert_eq!(Vector3i::new(5, 10, -5).max_axis(), Some(Vector3Axis::Y));
        assert_eq!(Vector3i::new(5, -5, 10).max_axis(), Some(Vector3Axis::Z));

        assert_eq!(Vector3i::new(-5, 5, 10).min_axis(), Some(Vector3Axis::X));
        assert_eq!(Vector3i::new(5, -5, 10).min_axis(), Some(Vector3Axis::Y));
        assert_eq!(Vector3i::new(5, 10, -5).min_axis(), Some(Vector3Axis::Z));

        assert_eq!(Vector3i::new(15, 15, 5).max_axis(), None);
        assert_eq!(Vector3i::new(15, 15, 15).max_axis(), None);
        assert_eq!(Vector3i::new(15, 15, 25).min_axis(), None);
        assert_eq!(Vector3i::new(15, 15, 15).min_axis(), None);

        // Checks for non-max / non-min equality "traps"
        assert_eq!(Vector3i::new(15, 15, 25).max_axis(), Some(Vector3Axis::Z));
        assert_eq!(Vector3i::new(15, 5, 15).min_axis(), Some(Vector3Axis::Y));
    }
}
