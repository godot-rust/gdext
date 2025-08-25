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
use crate::builtin::{inner, real, RVec4, Vector4, Vector4Axis};

/// Vector used for 4D math using integer coordinates.
///
/// 4-element structure that can be used to represent 4D grid coordinates or sets of integers.
///
/// `Vector4i` uses integer coordinates and is therefore preferable to [`Vector4`] when exact precision is
/// required. Note that the values are limited to 32 bits, and unlike `Vector4` this cannot be
/// configured with an engine build option. Use `i64` or [`PackedInt64Array`][crate::builtin::PackedInt64Array]
/// if 64-bit values are needed.
///
#[doc = shared_vector_docs!()]
///
/// ### Navigation to `impl` blocks within this page
///
/// - [Constants](#constants)
/// - [Constructors and general vector functions](#constructors-and-general-vector-functions)
/// - [Specialized `Vector4i` functions](#specialized-vector4i-functions)
/// - [4D functions](#4d-functions)
/// - [Trait impls + operators](#trait-implementations)
///
/// # All vector types
///
/// | Dimension | Floating-point                       | Integer                                |
/// |-----------|--------------------------------------|----------------------------------------|
/// | 2D        | [`Vector2`][crate::builtin::Vector2] | [`Vector2i`][crate::builtin::Vector2i] |
/// | 3D        | [`Vector3`][crate::builtin::Vector3] | [`Vector3i`][crate::builtin::Vector3i] |
/// | 4D        | [`Vector4`][crate::builtin::Vector4] | **`Vector4i`**                         |
///
/// # Godot docs
///
/// [`Vector4i` (stable)](https://docs.godotengine.org/en/stable/classes/class_vector4i.html)
#[derive(Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(C)]
pub struct Vector4i {
    /// The vector's X component.
    pub x: i32,

    /// The vector's Y component.
    pub y: i32,

    /// The vector's Z component.
    pub z: i32,

    /// The vector's W component.
    pub w: i32,
}

/// # Constants
impl Vector4i {
    impl_vector_consts!(i32);
    impl_integer_vector_consts!();
}

impl_vector_fns!(Vector4i, glam::IVec4, i32, (x, y, z, w));

/// # Specialized `Vector4i` functions
impl Vector4i {
    inline_impl_integer_vector_fns!(Vector4, x, y, z, w);

    /// Converts `self` to the corresponding [`real`] `glam` type.
    #[doc(hidden)]
    #[inline]
    pub fn to_glam_real(self) -> RVec4 {
        RVec4::new(
            self.x as real,
            self.y as real,
            self.z as real,
            self.w as real,
        )
    }

    #[doc(hidden)]
    #[inline]
    pub fn as_inner(&self) -> inner::InnerVector4i<'_> {
        inner::InnerVector4i::from_outer(self)
    }
}

impl_vector4x_fns!(Vector4i, i32);
impl_vector_operators!(Vector4i, i32, (x, y, z, w));

/// Formats the vector like Godot: `(x, y, z, w)`.
impl fmt::Display for Vector4i {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {}, {}, {})", self.x, self.y, self.z, self.w)
    }
}

// SAFETY:
// This type is represented as `Self` in Godot, so `*mut Self` is sound.
unsafe impl GodotFfi for Vector4i {
    const VARIANT_TYPE: ExtVariantType = ExtVariantType::Concrete(sys::VariantType::VECTOR4I);

    ffi_methods! { type sys::GDExtensionTypePtr = *mut Self; .. }
}

crate::meta::impl_godot_as_self!(Vector4i: ByValue);

impl GlamType for glam::IVec4 {
    type Mapped = Vector4i;

    fn to_front(&self) -> Self::Mapped {
        Vector4i::new(self.x, self.y, self.z, self.w)
    }

    fn from_front(mapped: &Self::Mapped) -> Self {
        glam::IVec4::new(mapped.x, mapped.y, mapped.z, mapped.w)
    }
}

impl GlamConv for Vector4i {
    type Glam = glam::IVec4;
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn coord_min_max() {
        let a = Vector4i::new(1, 3, 5, 0);
        let b = Vector4i::new(0, 5, 2, 1);
        assert_eq!(a.coord_min(b), Vector4i::new(0, 3, 2, 0),);
        assert_eq!(a.coord_max(b), Vector4i::new(1, 5, 5, 1));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_roundtrip() {
        let vector = Vector4i::default();
        let expected_json = "{\"x\":0,\"y\":0,\"z\":0,\"w\":0}";

        crate::builtin::test_utils::roundtrip(&vector, expected_json);
    }

    #[test]
    fn sign() {
        let vector = Vector4i::new(2, -5, 0, 999);
        assert_eq!(vector.sign(), Vector4i::new(1, -1, 0, 1));
    }

    #[test]
    fn axis_min_max() {
        assert_eq!(Vector4i::new(10, 5, -5, 0).max_axis(), Some(Vector4Axis::X));
        assert_eq!(Vector4i::new(5, 10, -5, 0).max_axis(), Some(Vector4Axis::Y));
        assert_eq!(Vector4i::new(5, -5, 10, 0).max_axis(), Some(Vector4Axis::Z));
        assert_eq!(Vector4i::new(5, -5, 0, 10).max_axis(), Some(Vector4Axis::W));

        assert_eq!(Vector4i::new(-5, 5, 10, 0).min_axis(), Some(Vector4Axis::X));
        assert_eq!(Vector4i::new(5, -5, 10, 0).min_axis(), Some(Vector4Axis::Y));
        assert_eq!(Vector4i::new(5, 10, -5, 0).min_axis(), Some(Vector4Axis::Z));
        assert_eq!(Vector4i::new(5, 10, 0, -5).min_axis(), Some(Vector4Axis::W));

        assert_eq!(Vector4i::new(15, 15, 5, -5).max_axis(), None);
        assert_eq!(Vector4i::new(15, 15, 15, 5).max_axis(), None);
        assert_eq!(Vector4i::new(15, 15, 15, 15).max_axis(), None);

        assert_eq!(Vector4i::new(15, 15, 25, 35).min_axis(), None);
        assert_eq!(Vector4i::new(15, 15, 15, 25).min_axis(), None);
        assert_eq!(Vector4i::new(15, 15, 15, 15).min_axis(), None);

        // Checks for non-max / non-min equality "traps"
        assert_eq!(Vector4i::new(5, 5, 25, 15).max_axis(), Some(Vector4Axis::Z));
        assert_eq!(
            Vector4i::new(15, 15, 5, -5).min_axis(),
            Some(Vector4Axis::W),
        );
    }

    #[test]
    fn test_iter_elementwise_prod() {
        let vecs = vec![Vector4i::new(1, 2, 3, 4), Vector4i::new(5, 6, 7, 8)];
        let expected = Vector4i::new(5, 12, 21, 32);
        let prod_refs: Vector4i = vecs.iter().product();
        let prod: Vector4i = vecs.into_iter().product();

        assert_eq!(prod_refs, expected);
        assert_eq!(prod, expected);
    }
}
