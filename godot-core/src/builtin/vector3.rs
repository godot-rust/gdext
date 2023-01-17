/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::fmt;

use godot_ffi as sys;
use sys::{ffi_methods, GodotFfi};

use crate::builtin::Vector3i;

/// Vector used for 3D math using floating point coordinates.
///
/// 2-element structure that can be used to represent positions in 2D space or any other pair of
/// numeric values.
///
/// It uses floating-point coordinates of 32-bit precision, unlike the engine's `float` type which
/// is always 64-bit. The engine can be compiled with the option `precision=double` to use 64-bit
/// vectors, but this is not yet supported in the `gdextension` crate.
/// 
/// See [`Vector3i`] for its integer counterpart.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct Vector3 {
    /// The vector's X component.
    pub x: f32,
    /// The vector's Y component.
    pub y: f32,
    /// The vector's Z component.
    pub z: f32,
}

impl_vector_operators!(Vector3, f32, (x, y, z));
impl_vector_index!(Vector3, f32, (x, y, z), Vector3Axis, (X, Y, Z));
impl_common_vector_fns!(Vector3, f32);
impl_float_vector_fns!(Vector3, f32);

impl Vector3 {
    /// Returns a `Vector3` with the given components.
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    /// Returns a new `Vector3` with all components set to `v`.
    pub const fn splat(v: f32) -> Self {
        Self { x: v, y: v, z: v }
    }

    /// Constructs a new `Vector3` from a [`Vector3i`].
    pub const fn from_vector3i(v: Vector3i) -> Self {
        Self { x: v.x as f32, y: v.y as f32, z: v.z as f32 }
    }

    /// Zero vector, a vector with all components set to `0.0`.
    pub const ZERO: Self = Self::splat(0.0);

    /// One vector, a vector with all components set to `1.0`.
    pub const ONE: Self = Self::splat(1.0);

    /// Infinity vector, a vector with all components set to `INFIINTY`.
    pub const INF: Self = Self::splat(f32::INFINITY);

    /// Left unit vector. Represents the local direction of left, and the global direction of west.
    pub const LEFT: Self = Self::new(-1.0, 0.0, 0.0);

    /// Right unit vector. Represents the local direction of right, and the global direction of east.
    pub const RIGHT: Self = Self::new(1.0, 0.0, 0.0);

    /// Up unit vector.
    pub const UP: Self = Self::new(0.0, -1.0, 0.0);

    /// Down unit vector.
    pub const DOWN: Self = Self::new(0.0, 1.0, 0.0);

    /// Forward unit vector. Represents the local direction of forward, and the global direction of north.
    pub const FORWARD: Self = Self::new(0.0, 0.0, -1.0);

    /// Back unit vector. Represents the local direction of back, and the global direction of south.
    pub const BACK: Self = Self::new(0.0, 0.0, 1.0);

    /// Converts the corresponding `glam` type to `Self`.
    fn from_glam(v: glam::Vec3) -> Self {
        Self::new(v.x, v.y, v.z)
    }

    /// Converts `self` to the corresponding `glam` type.
    fn to_glam(self) -> glam::Vec3 {
        glam::Vec3::new(self.x, self.y, self.z)
    }
}

/// Formats this vector in the same way the Godot engine would.
impl fmt::Display for Vector3 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {}, {})", self.x, self.y, self.z)
    }
}

impl GodotFfi for Vector3 {
    ffi_methods! { type sys::GDExtensionTypePtr = *mut Self; .. }
}

/// Enumerates the axes in a [`Vector3`].
// TODO auto-generate this, alongside all the other builtin type's enums
#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(i32)]
pub enum Vector3Axis {
    /// The X axis.
    X,
    /// The Y axis.
    Y,
    /// The Z axis.
    Z,
}

impl GodotFfi for Vector3Axis {
    ffi_methods! { type sys::GDExtensionTypePtr = *mut Self; .. }
}
