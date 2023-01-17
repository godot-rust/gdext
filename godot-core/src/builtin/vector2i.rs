/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::fmt;

use godot_ffi as sys;
use sys::{ffi_methods, GodotFfi};

use crate::builtin::Vector2;

/// Vector used for 2D math using integer coordinates.
///
/// 2-element structure that can be used to represent positions in 2D space or any other pair of
/// numeric values.
/// 
/// It uses integer coordinates and is therefore preferable to [`Vector2`] when exact precision is
/// required. Note that the values are limited to 32 bits, and unlike [`Vector2`] this cannot be
/// configured with an engine build option. Use `i64` or [`PackedInt64Array`] if 64-bit values are
/// needed.
#[derive(Debug, Default, Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
#[repr(C)]
pub struct Vector2i {
    /// The vector's X component.
    pub x: i32,
    /// The vector's Y component.
    pub y: i32,
}

impl_vector_operators!(Vector2i, i32, (x, y));
impl_vector_index!(Vector2i, i32, (x, y), Vector2iAxis, (X, Y));
impl_common_vector_fns!(Vector2i, i32);

impl Vector2i {
    /// Constructs a new `Vector2i` from the given `x` and `y`.
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// Constructs a new `Vector2i` with all components set to `v`.
    pub const fn splat(v: i32) -> Self {
        Self { x: v, y: v }
    }

    /// Constructs a new `Vector2i` from a [`Vector2`]. The floating point coordinates will be
    /// truncated.
    pub const fn from_vector2(v: Vector2) -> Self {
        Self { x: v.x as i32, y: v.y as i32 }
    }

    /// Zero vector, a vector with all components set to `0`.
    pub const ZERO: Self = Self::splat(0);

    /// One vector, a vector with all components set to `1`.
    pub const ONE: Self = Self::splat(1);

    /// Left unit vector. Represents the direction of left.
    pub const LEFT: Self = Self::new(-1, 0);

    /// Right unit vector. Represents the direction of right.
    pub const RIGHT: Self = Self::new(1, 0);

    /// Up unit vector. Y is down in 2D, so this vector points -Y.
    pub const UP: Self = Self::new(0, -1);

    /// Down unit vector. Y is down in 2D, so this vector points +Y.
    pub const DOWN: Self = Self::new(0, 1);

    /// Converts the corresponding `glam` type to `Self`.
    fn from_glam(v: glam::IVec2) -> Self {
        Self::new(v.x, v.y)
    }

    /// Converts `self` to the corresponding `glam` type.
    fn to_glam(self) -> glam::IVec2 {
        glam::IVec2::new(self.x, self.y)
    }
}

/// Formats this vector in the same way the Godot engine would.
impl fmt::Display for Vector2i {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

impl GodotFfi for Vector2i {
    ffi_methods! { type sys::GDExtensionTypePtr = *mut Self; .. }
}

/// Enumerates the axes in a [`Vector2i`].
#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(i32)]
pub enum Vector2iAxis {
    /// The X axis.
    X,
    /// The Y axis.
    Y,
}

impl GodotFfi for Vector2iAxis {
    ffi_methods! { type sys::GDExtensionTypePtr = *mut Self; .. }
}
