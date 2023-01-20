/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::fmt;

use godot_ffi as sys;
use sys::{ffi_methods, GodotFfi};

use crate::builtin::Vector4i;

/// Vector used for 4D math using floating point coordinates.
///
/// 4-element structure that can be used to represent any quadruplet of numeric values.
///
/// It uses floating-point coordinates of 32-bit precision, unlike the engine's `float` type which
/// is always 64-bit. The engine can be compiled with the option `precision=double` to use 64-bit
/// vectors, but this is not yet supported in the `gdextension` crate.
///
/// See [`Vector4i`] for its integer counterpart.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct Vector4 {
    /// The vector's X component.
    pub x: f32,
    /// The vector's Y component.
    pub y: f32,
    /// The vector's Z component.
    pub z: f32,
    /// The vector's W component.
    pub w: f32,
}

impl_vector_operators!(Vector4, f32, (x, y, z, w));
impl_vector_index!(Vector4, f32, (x, y, z, w), Vector4Axis, (X, Y, Z, W));
impl_common_vector_fns!(Vector4, f32);
impl_float_vector_fns!(Vector4, f32);

impl Vector4 {
    /// Returns a `Vector4` with the given components.
    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }

    /// Returns a new `Vector4` with all components set to `v`.
    pub const fn splat(v: f32) -> Self {
        Self::new(v, v, v, v)
    }

    /// Constructs a new `Vector3` from a [`Vector3i`].
    pub const fn from_vector4i(v: Vector4i) -> Self {
        Self {
            x: v.x as f32,
            y: v.y as f32,
            z: v.z as f32,
            w: v.w as f32,
        }
    }

    /// Zero vector, a vector with all components set to `0.0`.
    pub const ZERO: Self = Self::splat(0.0);

    /// One vector, a vector with all components set to `1.0`.
    pub const ONE: Self = Self::splat(1.0);

    /// Infinity vector, a vector with all components set to `f32::INFINITY`.
    pub const INF: Self = Self::splat(f32::INFINITY);

    /// Converts the corresponding `glam` type to `Self`.
    fn from_glam(v: glam::Vec4) -> Self {
        Self::new(v.x, v.y, v.z, v.w)
    }

    /// Converts `self` to the corresponding `glam` type.
    fn to_glam(self) -> glam::Vec4 {
        glam::Vec4::new(self.x, self.y, self.z, self.w)
    }
}

/// Formats the vector like Godot: `(x, y, z, w)`.
impl fmt::Display for Vector4 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {}, {}, {})", self.x, self.y, self.z, self.w)
    }
}

impl GodotFfi for Vector4 {
    ffi_methods! { type sys::GDExtensionTypePtr = *mut Self; .. }
}

/// Enumerates the axes in a [`Vector4`].
#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
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

impl GodotFfi for Vector4Axis {
    ffi_methods! { type sys::GDExtensionTypePtr = *mut Self; .. }
}
