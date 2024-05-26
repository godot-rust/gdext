/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot_ffi as sys;
use std::cmp::Ordering;
use sys::{ffi_methods, GodotFfi};

use crate::builtin::math::{FloatExt, GlamConv, GlamType};
use crate::builtin::meta::impl_godot_as_self;
use crate::builtin::{real, RVec2, Vector2, Vector2Axis};

use std::fmt;

/// Vector used for 2D math using integer coordinates.
///
/// 2-element structure that can be used to represent positions in 2D space or any other pair of
/// numeric values.
///
/// It uses integer coordinates and is therefore preferable to [`Vector2`] when exact precision is
/// required. Note that the values are limited to 32 bits, and unlike [`Vector2`] this cannot be
/// configured with an engine build option. Use `i64` or [`PackedInt64Array`][crate::builtin::PackedInt64Array]
/// if 64-bit values are needed.
#[derive(Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(C)]
pub struct Vector2i {
    /// The vector's X component.
    pub x: i32,

    /// The vector's Y component.
    pub y: i32,
}

impl Vector2i {
    /// Vector with all components set to `0`.
    pub const ZERO: Self = Self::splat(0);

    /// Vector with all components set to `1`.
    pub const ONE: Self = Self::splat(1);

    /// Unit vector in -X direction (right in 2D coordinate system).
    pub const LEFT: Self = Self::new(-1, 0);

    /// Unit vector in +X direction (right in 2D coordinate system).
    pub const RIGHT: Self = Self::new(1, 0);

    /// Unit vector in -Y direction (up in 2D coordinate system).
    pub const UP: Self = Self::new(0, -1);

    /// Unit vector in +Y direction (down in 2D coordinate system).
    pub const DOWN: Self = Self::new(0, 1);

    /// Constructs a new `Vector2i` from the given `x` and `y`.
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// Aspect ratio: x / y, as a `real` value.
    pub fn aspect(self) -> real {
        self.x as real / self.y as real
    }

    /// Axis of the vector's highest value. [`None`] if components are equal.
    pub fn max_axis(self) -> Option<Vector2Axis> {
        match self.x.cmp(&self.y) {
            Ordering::Less => Some(Vector2Axis::Y),
            Ordering::Equal => None,
            Ordering::Greater => Some(Vector2Axis::X),
        }
    }

    /// Axis of the vector's highest value. [`None`] if components are equal.
    pub fn min_axis(self) -> Option<Vector2Axis> {
        match self.x.cmp(&self.y) {
            Ordering::Less => Some(Vector2Axis::X),
            Ordering::Equal => None,
            Ordering::Greater => Some(Vector2Axis::Y),
        }
    }

    /// Constructs a new `Vector2i` with both components set to `v`.
    pub const fn splat(v: i32) -> Self {
        Self::new(v, v)
    }

    /// Constructs a new `Vector2i` from a [`Vector2`]. The floating point coordinates will be truncated.
    pub const fn from_vector2(v: Vector2) -> Self {
        Self {
            x: v.x as i32,
            y: v.y as i32,
        }
    }

    /// Converts the corresponding `glam` type to `Self`.
    fn from_glam(v: glam::IVec2) -> Self {
        Self::new(v.x, v.y)
    }

    /// Converts `self` to the corresponding `glam` type.
    fn to_glam(self) -> glam::IVec2 {
        glam::IVec2::new(self.x, self.y)
    }

    /// Converts `self` to the corresponding [`real`] `glam` type.
    fn to_glam_real(self) -> RVec2 {
        RVec2::new(self.x as real, self.y as real)
    }

    pub fn coords(&self) -> (i32, i32) {
        (self.x, self.y)
    }
}

/// Formats the vector like Godot: `(x, y)`.
impl fmt::Display for Vector2i {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

impl_common_vector_fns!(Vector2i, i32);
impl_integer_vector_glam_fns!(Vector2i, real);
impl_integer_vector_component_fns!(Vector2i, real, (x, y));
impl_vector_operators!(Vector2i, i32, (x, y));
impl_swizzle_trait_for_vector2x!(Vector2i, i32);

// SAFETY:
// This type is represented as `Self` in Godot, so `*mut Self` is sound.
unsafe impl GodotFfi for Vector2i {
    fn variant_type() -> sys::VariantType {
        sys::VariantType::VECTOR2I
    }

    ffi_methods! { type sys::GDExtensionTypePtr = *mut Self; .. }
}

impl_godot_as_self!(Vector2i);

impl GlamType for glam::IVec2 {
    type Mapped = Vector2i;

    fn to_front(&self) -> Self::Mapped {
        Vector2i::new(self.x, self.y)
    }

    fn from_front(mapped: &Self::Mapped) -> Self {
        glam::IVec2::new(mapped.x, mapped.y)
    }
}

impl GlamConv for Vector2i {
    type Glam = glam::IVec2;
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn coord_min_max() {
        let a = Vector2i::new(1, 3);
        let b = Vector2i::new(0, 5);
        assert_eq!(a.coord_min(b), Vector2i::new(0, 3));
        assert_eq!(a.coord_max(b), Vector2i::new(1, 5));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_roundtrip() {
        let vector = Vector2i::default();
        let expected_json = "{\"x\":0,\"y\":0}";

        crate::builtin::test_utils::roundtrip(&vector, expected_json);
    }

    #[test]
    fn axis_min_max() {
        assert_eq!(Vector2i::new(10, 5).max_axis(), Some(Vector2Axis::X));
        assert_eq!(Vector2i::new(5, 10).max_axis(), Some(Vector2Axis::Y));

        assert_eq!(Vector2i::new(-5, 5).min_axis(), Some(Vector2Axis::X));
        assert_eq!(Vector2i::new(5, -5).min_axis(), Some(Vector2Axis::Y));

        assert_eq!(Vector2i::new(15, 15).max_axis(), None);
        assert_eq!(Vector2i::new(15, 15).min_axis(), None);
    }
}
