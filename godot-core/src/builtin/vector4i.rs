/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::fmt;

use godot_ffi as sys;
use sys::{ffi_methods, GodotFfi};

use crate::builtin::Vector4;

/// Vector used for 4D math using integer coordinates.
///
/// 4-element structure that can be used to represent 4D grid coordinates or sets of integers.
///
/// It uses integer coordinates and is therefore preferable to [`Vector4`] when exact precision is
/// required. Note that the values are limited to 32 bits, and unlike [`Vector4`] this cannot be
/// configured with an engine build option. Use `i64` or [`PackedInt64Array`] if 64-bit values are
/// needed.
#[derive(Debug, Default, Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
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
impl_vector_index!(Vector4i, i32, (x, y, z, w), Vector4iAxis, (X, Y, Z, W));
impl_common_vector_fns!(Vector4i, i32);

impl Vector4i {
    /// Returns a `Vector4i` with the given components.
    pub const fn new(x: i32, y: i32, z: i32, w: i32) -> Self {
        Self { x, y, z, w }
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
}

/// Formats the vector like Godot: `(x, y, z, w)`.
impl fmt::Display for Vector4i {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {}, {}, {})", self.x, self.y, self.z, self.w)
    }
}

impl GodotFfi for Vector4i {
    ffi_methods! { type sys::GDExtensionTypePtr = *mut Self; .. }
}

/// Enumerates the axes in a [`Vector4i`].
#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(i32)]
pub enum Vector4iAxis {
    /// The X axis.
    X,
    /// The Y axis.
    Y,
    /// The Z axis.
    Z,
    /// The W axis.
    W,
}

impl GodotFfi for Vector4iAxis {
    ffi_methods! { type sys::GDExtensionTypePtr = *mut Self; .. }
}
