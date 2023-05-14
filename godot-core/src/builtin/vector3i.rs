/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::fmt;

use godot_ffi as sys;
use sys::{ffi_methods, GodotFfi};

use crate::builtin::Vector3;

use super::glam_helpers::{GlamConv, GlamType};
use super::IVec3;

/// Vector used for 3D math using integer coordinates.
///
/// 3-element structure that can be used to represent positions in 3D space or any other triple of
/// numeric values.
///
/// It uses integer coordinates and is therefore preferable to [`Vector3`] when exact precision is
/// required. Note that the values are limited to 32 bits, and unlike [`Vector3`] this cannot be
/// configured with an engine build option. Use `i64` or [`PackedInt64Array`] if 64-bit values are
/// needed.
#[derive(Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
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
    fn from_glam(v: IVec3) -> Self {
        Self::new(v.x, v.y, v.z)
    }

    /// Converts `self` to the corresponding `glam` type.
    fn to_glam(self) -> IVec3 {
        IVec3::new(self.x, self.y, self.z)
    }
}

/// Formats the vector like Godot: `(x, y, z)`.
impl fmt::Display for Vector3i {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {}, {})", self.x, self.y, self.z)
    }
}

impl_common_vector_fns!(Vector3i, i32);
impl_vector_operators!(Vector3i, i32, (x, y, z));
impl_vector_index!(Vector3i, i32, (x, y, z), Vector3iAxis, (X, Y, Z));

// SAFETY:
// This type is represented as `Self` in Godot, so `*mut Self` is sound.
unsafe impl GodotFfi for Vector3i {
    ffi_methods! { type sys::GDExtensionTypePtr = *mut Self; .. }
}

/// Enumerates the axes in a [`Vector3i`].
#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(i32)]
pub enum Vector3iAxis {
    /// The X axis.
    X,

    /// The Y axis.
    Y,

    /// The Z axis.
    Z,
}

// SAFETY:
// This type is represented as `Self` in Godot, so `*mut Self` is sound.
unsafe impl GodotFfi for Vector3iAxis {
    ffi_methods! { type sys::GDExtensionTypePtr = *mut Self; .. }
}

impl GlamType for IVec3 {
    type Mapped = Vector3i;

    fn to_front(&self) -> Self::Mapped {
        Vector3i::new(self.x, self.y, self.z)
    }

    fn from_front(mapped: &Self::Mapped) -> Self {
        IVec3::new(mapped.x, mapped.y, mapped.z)
    }
}

impl GlamConv for Vector3i {
    type Glam = IVec3;
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
}
