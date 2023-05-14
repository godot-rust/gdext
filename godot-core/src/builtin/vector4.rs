/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::fmt;

use godot_ffi as sys;
use sys::{ffi_methods, GodotFfi};

use crate::builtin::math::*;
use crate::builtin::Vector4i;

use super::glam_helpers::{GlamConv, GlamType};
use super::{real, RVec4};

/// Vector used for 4D math using floating point coordinates.
///
/// 4-element structure that can be used to represent any quadruplet of numeric values.
///
/// It uses floating-point coordinates of 32-bit precision, unlike the engine's `float` type which
/// is always 64-bit. The engine can be compiled with the option `precision=double` to use 64-bit
/// vectors; use the gdext library with the `double-precision` feature in that case.
///
/// See [`Vector4i`] for its integer counterpart.
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

impl_vector_operators!(Vector4, real, (x, y, z, w));
impl_vector_index!(Vector4, real, (x, y, z, w), Vector4Axis, (X, Y, Z, W));
impl_common_vector_fns!(Vector4, real);
impl_float_vector_fns!(Vector4, real);

impl Vector4 {
    /// Returns a `Vector4` with the given components.
    pub const fn new(x: real, y: real, z: real, w: real) -> Self {
        Self { x, y, z, w }
    }

    /// Returns a new `Vector4` with all components set to `v`.
    pub const fn splat(v: real) -> Self {
        Self::new(v, v, v, v)
    }

    /// Constructs a new `Vector3` from a [`Vector3i`].
    pub const fn from_vector4i(v: Vector4i) -> Self {
        Self {
            x: v.x as real,
            y: v.y as real,
            z: v.z as real,
            w: v.w as real,
        }
    }

    /// Zero vector, a vector with all components set to `0.0`.
    pub const ZERO: Self = Self::splat(0.0);

    /// One vector, a vector with all components set to `1.0`.
    pub const ONE: Self = Self::splat(1.0);

    /// Infinity vector, a vector with all components set to `real::INFINITY`.
    pub const INF: Self = Self::splat(real::INFINITY);

    /// Converts the corresponding `glam` type to `Self`.
    fn from_glam(v: RVec4) -> Self {
        Self::new(v.x, v.y, v.z, v.w)
    }

    /// Converts `self` to the corresponding `glam` type.
    fn to_glam(self) -> RVec4 {
        RVec4::new(self.x, self.y, self.z, self.w)
    }

    pub fn is_equal_approx(self, to: Self) -> bool {
        is_equal_approx(self.x, to.x)
            && is_equal_approx(self.y, to.y)
            && is_equal_approx(self.z, to.z)
            && is_equal_approx(self.w, to.w)
    }
}

/// Formats the vector like Godot: `(x, y, z, w)`.
impl fmt::Display for Vector4 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {}, {}, {})", self.x, self.y, self.z, self.w)
    }
}

// SAFETY:
// This type is represented as `Self` in Godot, so `*mut Self` is sound.
unsafe impl GodotFfi for Vector4 {
    ffi_methods! { type sys::GDExtensionTypePtr = *mut Self; .. }
}

/// Enumerates the axes in a [`Vector4`].
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
#[repr(i32)]
pub enum Vector4Axis {
    /// The X axis.
    X,

    /// The Y axis.
    Y,

    /// The Z axis.
    Z,

    /// The W axis.
    W,
}

// SAFETY:
// This type is represented as `Self` in Godot, so `*mut Self` is sound.
unsafe impl GodotFfi for Vector4Axis {
    ffi_methods! { type sys::GDExtensionTypePtr = *mut Self; .. }
}

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
    use crate::assert_eq_approx;

    use super::*;

    #[test]
    fn coord_min_max() {
        let a = Vector4::new(1.2, 3.4, 5.6, 0.1);
        let b = Vector4::new(0.1, 5.6, 2.3, 1.2);
        assert_eq_approx!(
            a.coord_min(b),
            Vector4::new(0.1, 3.4, 2.3, 0.1),
            Vector4::is_equal_approx
        );
        assert_eq_approx!(
            a.coord_max(b),
            Vector4::new(1.2, 5.6, 5.6, 1.2),
            Vector4::is_equal_approx
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_roundtrip() {
        let vector = Vector4::default();
        let expected_json = "{\"x\":0.0,\"y\":0.0,\"z\":0.0,\"w\":0.0}";

        crate::builtin::test_utils::roundtrip(&vector, expected_json);
    }
}
