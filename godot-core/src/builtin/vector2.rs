/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::fmt;

use godot_ffi as sys;
use sys::{ffi_methods, GodotFfi};

use crate::builtin::Vector2i;

/// Vector used for 2D math using floating point coordinates.
///
/// 2-element structure that can be used to represent positions in 2D space or any other pair of
/// numeric values.
///
/// It uses floating-point coordinates of 32-bit precision, unlike the engine's `float` type which
/// is always 64-bit. The engine can be compiled with the option `precision=double` to use 64-bit
/// vectors, but this is not yet supported in the `gdextension` crate.
///
/// See [`Vector2i`] for its integer counterpart.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct Vector2 {
    /// The vector's X component.
    pub x: f32,
    /// The vector's Y component.
    pub y: f32,
}

impl Vector2 {
    /// Vector with all components set to `0.0`.
    pub const ZERO: Self = Self::splat(0.0);

    /// Vector with all components set to `1.0`.
    pub const ONE: Self = Self::splat(1.0);

    /// Vector with all components set to `f32::INFINITY`.
    pub const INF: Self = Self::splat(f32::INFINITY);

    /// Unit vector in -X direction (right in 2D coordinate system).
    pub const LEFT: Self = Self::new(-1.0, 0.0);

    /// Unit vector in +X direction (right in 2D coordinate system).
    pub const RIGHT: Self = Self::new(1.0, 0.0);

    /// Unit vector in -Y direction (up in 2D coordinate system).
    pub const UP: Self = Self::new(0.0, -1.0);

    /// Unit vector in +Y direction (down in 2D coordinate system).
    pub const DOWN: Self = Self::new(0.0, 1.0);

    /// Constructs a new `Vector2` from the given `x` and `y`.
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Constructs a new `Vector2` with both components set to `v`.
    pub const fn splat(v: f32) -> Self {
        Self::new(v, v)
    }

    /// Constructs a new `Vector2` from a [`Vector2i`].
    pub const fn from_vector2i(v: Vector2i) -> Self {
        Self {
            x: v.x as f32,
            y: v.y as f32,
        }
    }

    /// Returns the result of rotating this vector by `angle` (in radians).
    pub fn rotated(self, angle: f32) -> Self {
        Self::from_glam(glam::Affine2::from_angle(angle).transform_vector2(self.to_glam()))
    }

    /// Converts the corresponding `glam` type to `Self`.
    fn from_glam(v: glam::Vec2) -> Self {
        Self::new(v.x, v.y)
    }

    /// Converts `self` to the corresponding `glam` type.
    fn to_glam(self) -> glam::Vec2 {
        glam::Vec2::new(self.x, self.y)
    }
}

/// Formats the vector like Godot: `(x, y)`.
impl fmt::Display for Vector2 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

impl_common_vector_fns!(Vector2, f32);
impl_float_vector_fns!(Vector2, f32);
impl_vector_operators!(Vector2, f32, (x, y));
impl_vector_index!(Vector2, f32, (x, y), Vector2Axis, (X, Y));

impl GodotFfi for Vector2 {
    ffi_methods! { type sys::GDExtensionTypePtr = *mut Self; .. }
}

/// Enumerates the axes in a [`Vector2`].
#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(i32)]
pub enum Vector2Axis {
    /// The X axis.
    X,
    /// The Y axis.
    Y,
}

impl GodotFfi for Vector2Axis {
    ffi_methods! { type sys::GDExtensionTypePtr = *mut Self; .. }
}
