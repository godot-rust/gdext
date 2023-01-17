/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::fmt;

use godot_ffi as sys;
use sys::{ffi_methods, GodotFfi};

use crate::builtin::Vector3;

/// Vector used for 3D math using integer coordinates.
///
/// 3-element structure that can be used to represent positions in 3D space or any other pair of
/// numeric values.
/// 
/// It uses integer coordinates and is therefore preferable to [`Vector3`] when exact precision is
/// required. Note that the values are limited to 32 bits, and unlike [`Vector3`] this cannot be
/// configured with an engine build option. Use `i64` or [`PackedInt64Array`] if 64-bit values are
/// needed.
#[derive(Debug, Default, Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
#[repr(C)]
pub struct Vector3i {
    /// The vector's X component.
    pub x: i32,
    /// The vector's Y component.
    pub y: i32,
    /// The vector's Z component.
    pub z: i32,
}

impl_vector_operators!(Vector3i, i32, (x, y, z));
impl_vector_index!(Vector3i, i32, (x, y, z), Vector3iAxis, (X, Y, Z));
impl_common_vector_fns!(Vector3i, i32);

impl Vector3i {
    /// Returns a `Vector3i` with the given components.
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    /// Constructs a new `Vector3i` with all components set to `v`.
    pub const fn splat(v: i32) -> Self {
        Self { x: v, y: v, z: v }
    }

    /// Constructs a new `Vector3i` from a [`Vector3`]. The floating point coordinates will be
    /// truncated.
    pub const fn from_vector3(v: Vector3) -> Self {
        Self { x: v.x as i32, y: v.y as i32, z: v.z as i32 }
    }

    /// Zero vector, a vector with all components set to `0`.
    pub const ZERO: Self = Self::splat(0);

    /// One vector, a vector with all components set to `1`.
    pub const ONE: Self = Self::splat(1);

    /// Left unit vector. Represents the local direction of left, and the global direction of west.
    pub const LEFT: Self = Self::new(-1, 0, 0);

    /// Right unit vector. Represents the local direction of right, and the global direction of east.
    pub const RIGHT: Self = Self::new(1, 0, 0);

    /// Up unit vector.
    pub const UP: Self = Self::new(0, -1, 0);

    /// Down unit vector.
    pub const DOWN: Self = Self::new(0, 1, 0);

    /// Forward unit vector. Represents the local direction of forward, and the global direction of north.
    pub const FORWARD: Self = Self::new(0, 0, -1);

    /// Back unit vector. Represents the local direction of back, and the global direction of south.
    pub const BACK: Self = Self::new(0, 0, 1);

    /// Converts the corresponding `glam` type to `Self`.
    fn from_glam(v: glam::IVec3) -> Self {
        Self::new(v.x, v.y, v.z)
    }

    /// Converts `self` to the corresponding `glam` type.
    fn to_glam(self) -> glam::IVec3 {
        glam::IVec3::new(self.x, self.y, self.z)
    }
}

/// Formats this vector in the same way the Godot engine would.
impl fmt::Display for Vector3i {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {}, {})", self.x, self.y, self.z)
    }
}

impl GodotFfi for Vector3i {
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

impl GodotFfi for Vector3iAxis {
    ffi_methods! { type sys::GDExtensionTypePtr = *mut Self; .. }
}
