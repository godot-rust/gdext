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

use crate::builtin::math::{FloatExt, GlamConv, GlamType};
use crate::builtin::meta::impl_godot_as_self;
use crate::builtin::{real, RVec3, Vector3, Vector3Axis};

/// Vector used for 3D math using integer coordinates.
///
/// 3-element structure that can be used to represent positions in 3D space or any other triple of
/// numeric values.
///
/// It uses integer coordinates and is therefore preferable to [`Vector3`] when exact precision is
/// required. Note that the values are limited to 32 bits, and unlike [`Vector3`] this cannot be
/// configured with an engine build option. Use `i64` or [`PackedInt64Array`][crate::builtin::PackedInt64Array]
/// if 64-bit values are needed.
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

impl Vector3i {
    /// Vector with all components set to `0`.
    pub const ZERO: Self = Self::splat(0);

    /// Vector with all components set to `1`.
    pub const ONE: Self = Self::splat(1);

    /// Unit vector in -X direction.
    pub const LEFT: Self = Self::new(-1, 0, 0);

    /// Unit vector in +X direction.
    pub const RIGHT: Self = Self::new(1, 0, 0);

    /// Unit vector in +Y direction.
    pub const UP: Self = Self::new(0, 1, 0);

    /// Unit vector in -Y direction.
    pub const DOWN: Self = Self::new(0, -1, 0);

    /// Unit vector in -Z direction.
    pub const FORWARD: Self = Self::new(0, 0, -1);

    /// Unit vector in +Z direction.
    pub const BACK: Self = Self::new(0, 0, 1);

    /// Returns a `Vector3i` with the given components.
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    /// Axis of the vector's highest value. [`None`] if at least two components are equal.
    pub fn max_axis(self) -> Option<Vector3Axis> {
        use Vector3Axis::*;

        match self.x.cmp(&self.y) {
            Ordering::Less => match self.y.cmp(&self.z) {
                Ordering::Less => Some(Z),
                Ordering::Equal => None,
                Ordering::Greater => Some(Y),
            },
            Ordering::Equal => match self.x.cmp(&self.z) {
                Ordering::Less => Some(Z),
                _ => None,
            },
            Ordering::Greater => match self.x.cmp(&self.z) {
                Ordering::Less => Some(Z),
                Ordering::Equal => None,
                Ordering::Greater => Some(X),
            },
        }
    }

    /// Axis of the vector's highest value. [`None`] if at least two components are equal.
    pub fn min_axis(self) -> Option<Vector3Axis> {
        use Vector3Axis::*;

        match self.x.cmp(&self.y) {
            Ordering::Less => match self.x.cmp(&self.z) {
                Ordering::Less => Some(X),
                Ordering::Equal => None,
                Ordering::Greater => Some(Z),
            },
            Ordering::Equal => match self.x.cmp(&self.z) {
                Ordering::Greater => Some(Z),
                _ => None,
            },
            Ordering::Greater => match self.y.cmp(&self.z) {
                Ordering::Less => Some(Y),
                Ordering::Equal => None,
                Ordering::Greater => Some(Z),
            },
        }
    }

    /// Constructs a new `Vector3i` with all components set to `v`.
    pub const fn splat(v: i32) -> Self {
        Self::new(v, v, v)
    }

    /// Constructs a new `Vector3i` from a [`Vector3`]. The floating point coordinates will be truncated.
    pub const fn from_vector3(v: Vector3) -> Self {
        Self {
            x: v.x as i32,
            y: v.y as i32,
            z: v.z as i32,
        }
    }

    /// Converts the corresponding `glam` type to `Self`.
    fn from_glam(v: glam::IVec3) -> Self {
        Self::new(v.x, v.y, v.z)
    }

    /// Converts `self` to the corresponding `glam` type.
    fn to_glam(self) -> glam::IVec3 {
        glam::IVec3::new(self.x, self.y, self.z)
    }

    /// Converts `self` to the corresponding [`real`] `glam` type.
    fn to_glam_real(self) -> RVec3 {
        RVec3::new(self.x as real, self.y as real, self.z as real)
    }

    pub fn coords(&self) -> (i32, i32, i32) {
        (self.x, self.y, self.z)
    }
}

/// Formats the vector like Godot: `(x, y, z)`.
impl fmt::Display for Vector3i {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {}, {})", self.x, self.y, self.z)
    }
}

impl_common_vector_fns!(Vector3i, i32);
impl_integer_vector_glam_fns!(Vector3i, real);
impl_integer_vector_component_fns!(Vector3i, real, (x, y, z));
impl_vector_operators!(Vector3i, i32, (x, y, z));
impl_from_tuple_for_vector3x!(Vector3i, i32);

// SAFETY:
// This type is represented as `Self` in Godot, so `*mut Self` is sound.
unsafe impl GodotFfi for Vector3i {
    fn variant_type() -> sys::VariantType {
        sys::VariantType::Vector3i
    }

    ffi_methods! { type sys::GDExtensionTypePtr = *mut Self; .. }
}

impl_godot_as_self!(Vector3i);

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
