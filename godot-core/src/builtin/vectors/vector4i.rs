/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot_ffi as sys;
use sys::{ffi_methods, GodotFfi};

use crate::builtin::math::{FloatExt, GlamConv, GlamType};
use crate::builtin::meta::impl_godot_as_self;
use crate::builtin::{real, RVec4, Vector4, Vector4Axis};

use std::fmt;

/// Vector used for 4D math using integer coordinates.
///
/// 4-element structure that can be used to represent 4D grid coordinates or sets of integers.
///
/// It uses integer coordinates and is therefore preferable to [`Vector4`] when exact precision is
/// required. Note that the values are limited to 32 bits, and unlike [`Vector4`] this cannot be
/// configured with an engine build option. Use `i64` or [`PackedInt64Array`][crate::builtin::PackedInt64Array]
/// if 64-bit values are needed.
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

impl_vector_operators!(Vector4i, i32, (x, y, z, w));
impl_integer_vector_glam_fns!(Vector4i, real);
impl_integer_vector_component_fns!(Vector4i, real, (x, y, z, w));
impl_common_vector_fns!(Vector4i, i32);
impl_from_tuple_for_vector4x!(Vector4i, i32);

impl Vector4i {
    /// Returns a `Vector4i` with the given components.
    pub const fn new(x: i32, y: i32, z: i32, w: i32) -> Self {
        Self { x, y, z, w }
    }

    /// Axis of the vector's highest value. [`None`] if at least two components are equal.
    pub fn max_axis(self) -> Option<Vector4Axis> {
        use Vector4Axis::*;

        let mut max_axis = X;
        let mut previous = None;
        let mut max_value = self.x;

        let components = [(Y, self.y), (Z, self.z), (W, self.w)];

        for (axis, value) in components {
            if value >= max_value {
                max_axis = axis;
                previous = Some(max_value);
                max_value = value;
            }
        }

        (Some(max_value) != previous).then_some(max_axis)
    }

    /// Axis of the vector's highest value. [`None`] if at least two components are equal.
    pub fn min_axis(self) -> Option<Vector4Axis> {
        use Vector4Axis::*;

        let mut min_axis = X;
        let mut previous = None;
        let mut min_value = self.x;

        let components = [(Y, self.y), (Z, self.z), (W, self.w)];

        for (axis, value) in components {
            if value <= min_value {
                min_axis = axis;
                previous = Some(min_value);
                min_value = value;
            }
        }

        (Some(min_value) != previous).then_some(min_axis)
    }

    /// Constructs a new `Vector4i` with all components set to `v`.
    pub const fn splat(v: i32) -> Self {
        Self::new(v, v, v, v)
    }

    /// Constructs a new `Vector4i` from a [`Vector4`]. The floating point coordinates will be
    /// truncated.
    pub const fn from_vector3(v: Vector4) -> Self {
        Self {
            x: v.x as i32,
            y: v.y as i32,
            z: v.z as i32,
            w: v.w as i32,
        }
    }

    /// Zero vector, a vector with all components set to `0`.
    pub const ZERO: Self = Self::splat(0);

    /// One vector, a vector with all components set to `1`.
    pub const ONE: Self = Self::splat(1);

    /// Converts the corresponding `glam` type to `Self`.
    fn from_glam(v: glam::IVec4) -> Self {
        Self::new(v.x, v.y, v.z, v.w)
    }

    /// Converts `self` to the corresponding `glam` type.
    fn to_glam(self) -> glam::IVec4 {
        glam::IVec4::new(self.x, self.y, self.z, self.w)
    }

    /// Converts `self` to the corresponding [`real`] `glam` type.
    fn to_glam_real(self) -> RVec4 {
        RVec4::new(
            self.x as real,
            self.y as real,
            self.z as real,
            self.w as real,
        )
    }

    pub fn coords(&self) -> (i32, i32, i32, i32) {
        (self.x, self.y, self.z, self.w)
    }
}

/// Formats the vector like Godot: `(x, y, z, w)`.
impl fmt::Display for Vector4i {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {}, {}, {})", self.x, self.y, self.z, self.w)
    }
}

// SAFETY:
// This type is represented as `Self` in Godot, so `*mut Self` is sound.
unsafe impl GodotFfi for Vector4i {
    fn variant_type() -> sys::VariantType {
        sys::VariantType::Vector4i
    }

    ffi_methods! { type sys::GDExtensionTypePtr = *mut Self; .. }
}

impl_godot_as_self!(Vector4i);

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
}
