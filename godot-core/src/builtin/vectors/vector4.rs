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

use crate::builtin::math::{FloatExt, GlamConv, GlamType};
use crate::builtin::{inner, real, RVec4, Vector4Axis, Vector4i};

/// Vector used for 4D math using floating point coordinates.
///
/// 4-element structure that can be used to represent any quadruplet of numeric values.
///
/// It uses floating-point coordinates of 32-bit precision, unlike the engine's `float` type which
/// is always 64-bit. The engine can be compiled with the option `precision=double` to use 64-bit
/// vectors; use the gdext library with the `double-precision` feature in that case.
///
#[doc = shared_vector_docs!()]
///
/// ### Navigation to `impl` blocks within this page
///
/// - [Constants](#constants)
/// - [Constructors and general vector functions](#constructors-and-general-vector-functions)
/// - [Specialized `Vector4` functions](#specialized-vector4-functions)
/// - [Float-specific functions](#float-specific-functions)
/// - [4D functions](#4d-functions)
/// - [3D and 4D functions](#3d-and-4d-functions)
/// - [Trait impls + operators](#trait-implementations)
///
/// # All vector types
///
/// | Dimension | Floating-point                       | Integer                                |
/// |-----------|--------------------------------------|----------------------------------------|
/// | 2D        | [`Vector2`][crate::builtin::Vector2] | [`Vector2i`][crate::builtin::Vector2i] |
/// | 3D        | [`Vector3`][crate::builtin::Vector3] | [`Vector3i`][crate::builtin::Vector3i] |
/// | 4D        | **`Vector4`**                        | [`Vector4i`][crate::builtin::Vector4i] |
///
/// # Godot docs
///
/// [`Vector4` (stable)](https://docs.godotengine.org/en/stable/classes/class_vector4.html)
#[derive(Default, Copy, Clone, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(C)]
pub struct Vector4 {
    /// The vector's X component.
    pub x: real,

    /// The vector's Y component.
    pub y: real,

    /// The vector's Z component.
    pub z: real,

    /// The vector's W component.
    pub w: real,
}

/// # Constants
impl Vector4 {
    impl_vector_consts!(real);
    impl_float_vector_consts!();
}

impl_vector_fns!(Vector4, RVec4, real, (x, y, z, w));

/// # Specialized `Vector4` functions
impl Vector4 {
    #[doc(hidden)]
    #[inline]
    pub fn as_inner(&self) -> inner::InnerVector4<'_> {
        inner::InnerVector4::from_outer(self)
    }
}

impl_float_vector_fns!(Vector4, Vector4i, (x, y, z, w));
impl_vector4x_fns!(Vector4, real);
impl_vector3_vector4_fns!(Vector4, (x, y, z, w));

impl_vector_operators!(Vector4, real, (x, y, z, w));

/// Formats the vector like Godot: `(x, y, z, w)`.
impl fmt::Display for Vector4 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {}, {}, {})", self.x, self.y, self.z, self.w)
    }
}

// SAFETY:
// This type is represented as `Self` in Godot, so `*mut Self` is sound.
unsafe impl GodotFfi for Vector4 {
    const VARIANT_TYPE: ExtVariantType = ExtVariantType::Concrete(sys::VariantType::VECTOR4);

    ffi_methods! { type sys::GDExtensionTypePtr = *mut Self; .. }
}

crate::meta::impl_godot_as_self!(Vector4: ByValue);

impl GlamType for RVec4 {
    type Mapped = Vector4;

    fn to_front(&self) -> Self::Mapped {
        Vector4::new(self.x, self.y, self.z, self.w)
    }

    fn from_front(mapped: &Self::Mapped) -> Self {
        RVec4::new(mapped.x, mapped.y, mapped.z, mapped.w)
    }
}

impl GlamConv for Vector4 {
    type Glam = RVec4;
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::builtin::math::assert_eq_approx;

    #[test]
    fn coord_min_max() {
        let a = Vector4::new(1.2, 3.4, 5.6, 0.1);
        let b = Vector4::new(0.1, 5.6, 2.3, 1.2);
        assert_eq_approx!(a.coord_min(b), Vector4::new(0.1, 3.4, 2.3, 0.1),);
        assert_eq_approx!(a.coord_max(b), Vector4::new(1.2, 5.6, 5.6, 1.2),);
    }

    #[test]
    fn sign() {
        let vector = Vector4::new(0.2, -0.5, 0., 999.0);
        assert_eq!(vector.sign(), Vector4::new(1., -1., 0., 1.));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_roundtrip() {
        let vector = Vector4::default();
        let expected_json = "{\"x\":0.0,\"y\":0.0,\"z\":0.0,\"w\":0.0}";

        crate::builtin::test_utils::roundtrip(&vector, expected_json);
    }
}
