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
/// 3-element structure that can be used to represent positions in 2D space or any other triple of
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

impl Vector3 {
    /// Vector with all components set to `0.0`.
    pub const ZERO: Self = Self::splat(0.0);

    /// Vector with all components set to `1.0`.
    pub const ONE: Self = Self::splat(1.0);

    /// Unit vector in -X direction. Can be interpreted as left in an untransformed 3D world.
    pub const LEFT: Self = Self::new(-1.0, 0.0, 0.0);

    /// Unit vector in +X direction. Can be interpreted as right in an untransformed 3D world.
    pub const RIGHT: Self = Self::new(1.0, 0.0, 0.0);

    /// Unit vector in -Y direction. Typically interpreted as down in a 3D world.
    pub const UP: Self = Self::new(0.0, -1.0, 0.0);

    /// Unit vector in +Y direction. Typically interpreted as up in a 3D world.
    pub const DOWN: Self = Self::new(0.0, 1.0, 0.0);

    /// Unit vector in -Z direction. Can be interpreted as "into the screen" in an untransformed 3D world.
    pub const FORWARD: Self = Self::new(0.0, 0.0, -1.0);

    /// Unit vector in +Z direction. Can be interpreted as "out of the screen" in an untransformed 3D world.
    pub const BACK: Self = Self::new(0.0, 0.0, 1.0);

    /// Returns a `Vector3` with the given components.
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    /// Returns a new `Vector3` with all components set to `v`.
    pub const fn splat(v: f32) -> Self {
        Self::new(v, v, v)
    }

    /// Constructs a new `Vector3` from a [`Vector3i`].
    pub const fn from_vector3i(v: Vector3i) -> Self {
        Self {
            x: v.x as f32,
            y: v.y as f32,
            z: v.z as f32,
        }
    }

    /// Converts the corresponding `glam` type to `Self`.
    fn from_glam(v: glam::Vec3) -> Self {
        Self::new(v.x, v.y, v.z)
    }

    /// Converts `self` to the corresponding `glam` type.
    fn to_glam(self) -> glam::Vec3 {
        glam::Vec3::new(self.x, self.y, self.z)
    }
}

/// Formats the vector like Godot: `(x, y, z)`.
impl fmt::Display for Vector3 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {}, {})", self.x, self.y, self.z)
    }
}

impl_common_vector_fns!(Vector3, f32);
impl_float_vector_fns!(Vector3, f32);
impl_vector_operators!(Vector3, f32, (x, y, z));
impl_vector_index!(Vector3, f32, (x, y, z), Vector3Axis, (X, Y, Z));

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
