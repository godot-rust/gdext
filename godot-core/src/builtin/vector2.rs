/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
use std::fmt;
use std::ops::*;

use godot_ffi as sys;
use sys::{ffi_methods, GodotFfi};

use crate::builtin::math::*;
use crate::builtin::{inner, Vector2i};

use super::glam_helpers::GlamConv;
use super::glam_helpers::GlamType;
use super::{real, RAffine2, RVec2};

/// Vector used for 2D math using floating point coordinates.
///
/// 2-element structure that can be used to represent positions in 2D space or any other pair of
/// numeric values.
///
/// It uses floating-point coordinates of 32-bit precision, unlike the engine's `float` type which
/// is always 64-bit. The engine can be compiled with the option `precision=double` to use 64-bit
/// vectors; use the gdext library with the `double-precision` feature in that case.
///
/// See [`Vector2i`] for its integer counterpart.
#[derive(Default, Copy, Clone, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(C)]
pub struct Vector2 {
    /// The vector's X component.
    pub x: real,

    /// The vector's Y component.
    pub y: real,
}

impl Vector2 {
    /// Vector with all components set to `0.0`.
    pub const ZERO: Self = Self::splat(0.0);

    /// Vector with all components set to `1.0`.
    pub const ONE: Self = Self::splat(1.0);

    /// Vector with all components set to `real::INFINITY`.
    pub const INF: Self = Self::splat(real::INFINITY);

    /// Unit vector in -X direction (right in 2D coordinate system).
    pub const LEFT: Self = Self::new(-1.0, 0.0);

    /// Unit vector in +X direction (right in 2D coordinate system).
    pub const RIGHT: Self = Self::new(1.0, 0.0);

    /// Unit vector in -Y direction (up in 2D coordinate system).
    pub const UP: Self = Self::new(0.0, -1.0);

    /// Unit vector in +Y direction (down in 2D coordinate system).
    pub const DOWN: Self = Self::new(0.0, 1.0);

    /// Constructs a new `Vector2` from the given `x` and `y`.
    pub const fn new(x: real, y: real) -> Self {
        Self { x, y }
    }

    /// Constructs a new `Vector2` with both components set to `v`.
    pub const fn splat(v: real) -> Self {
        Self::new(v, v)
    }

    /// Constructs a new `Vector2` from a [`Vector2i`].
    pub const fn from_vector2i(v: Vector2i) -> Self {
        Self {
            x: v.x as real,
            y: v.y as real,
        }
    }

    /// Converts the corresponding `glam` type to `Self`.
    fn from_glam(v: RVec2) -> Self {
        Self::new(v.x, v.y)
    }

    /// Converts `self` to the corresponding `glam` type.
    fn to_glam(self) -> RVec2 {
        RVec2::new(self.x, self.y)
    }

    pub fn angle(self) -> real {
        self.y.atan2(self.x)
    }

    pub fn angle_to(self, to: Self) -> real {
        self.to_glam().angle_between(to.to_glam())
    }

    pub fn angle_to_point(self, to: Self) -> real {
        (to - self).angle()
    }

    pub fn aspect(self) -> real {
        self.x / self.y
    }

    pub fn lerp(self, other: Self, weight: real) -> Self {
        Self::new(lerp(self.x, other.x, weight), lerp(self.y, other.y, weight))
    }

    pub fn bezier_derivative(self, control_1: Self, control_2: Self, end: Self, t: real) -> Self {
        let x = bezier_derivative(self.x, control_1.x, control_2.x, end.x, t);
        let y = bezier_derivative(self.y, control_1.y, control_2.y, end.y, t);

        Self::new(x, y)
    }

    pub fn bezier_interpolate(self, control_1: Self, control_2: Self, end: Self, t: real) -> Self {
        let x = bezier_interpolate(self.x, control_1.x, control_2.x, end.x, t);
        let y = bezier_interpolate(self.y, control_1.y, control_2.y, end.y, t);

        Self::new(x, y)
    }

    pub fn bounce(self, normal: Self) -> Self {
        -self.reflect(normal)
    }

    pub fn ceil(self) -> Self {
        Self::from_glam(self.to_glam().ceil())
    }

    pub fn clamp(self, min: Self, max: Self) -> Self {
        Self::from_glam(self.to_glam().clamp(min.to_glam(), max.to_glam()))
    }

    pub fn cross(self, with: Self) -> real {
        self.to_glam().perp_dot(with.to_glam())
    }

    pub fn cubic_interpolate(self, b: Self, pre_a: Self, post_b: Self, weight: real) -> Self {
        let x = cubic_interpolate(self.x, b.x, pre_a.x, post_b.x, weight);
        let y = cubic_interpolate(self.y, b.y, pre_a.y, post_b.y, weight);

        Self::new(x, y)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn cubic_interpolate_in_time(
        self,
        b: Self,
        pre_a: Self,
        post_b: Self,
        weight: real,
        b_t: real,
        pre_a_t: real,
        post_b_t: real,
    ) -> Self {
        let x = cubic_interpolate_in_time(
            self.x, b.x, pre_a.x, post_b.x, weight, b_t, pre_a_t, post_b_t,
        );
        let y = cubic_interpolate_in_time(
            self.y, b.y, pre_a.y, post_b.y, weight, b_t, pre_a_t, post_b_t,
        );

        Self::new(x, y)
    }

    pub fn direction_to(self, to: Self) -> Self {
        (to - self).normalized()
    }

    pub fn distance_squared_to(self, to: Self) -> real {
        (to - self).length_squared()
    }

    pub fn distance_to(self, to: Self) -> real {
        (to - self).length()
    }

    pub fn dot(self, other: Self) -> real {
        self.to_glam().dot(other.to_glam())
    }

    pub fn floor(self) -> Self {
        Self::from_glam(self.to_glam().floor())
    }

    pub fn from_angle(angle: real) -> Self {
        Self::from_glam(RVec2::from_angle(angle))
    }

    pub fn is_equal_approx(self, to: Self) -> bool {
        is_equal_approx(self.x, to.x) && is_equal_approx(self.y, to.y)
    }

    pub fn is_finite(self) -> bool {
        self.to_glam().is_finite()
    }

    pub fn is_normalized(self) -> bool {
        self.to_glam().is_normalized()
    }

    pub fn is_zero_approx(self) -> bool {
        is_zero_approx(self.x) && is_zero_approx(self.y)
    }

    pub fn length_squared(self) -> real {
        self.to_glam().length_squared()
    }

    pub fn limit_length(self, length: Option<real>) -> Self {
        Self::from_glam(self.to_glam().clamp_length_max(length.unwrap_or(1.0)))
    }

    pub fn max_axis_index(self) -> Vector2Axis {
        if self.x < self.y {
            Vector2Axis::Y
        } else {
            Vector2Axis::X
        }
    }

    pub fn min_axis_index(self) -> Vector2Axis {
        if self.x < self.y {
            Vector2Axis::X
        } else {
            Vector2Axis::Y
        }
    }

    pub fn move_toward(self, to: Self, delta: real) -> Self {
        let vd = to - self;
        let len = vd.length();
        if len <= delta || len < CMP_EPSILON {
            to
        } else {
            self + vd / len * delta
        }
    }

    pub fn orthogonal(self) -> Self {
        Self::new(self.y, -self.x)
    }

    pub fn posmod(self, pmod: real) -> Self {
        Self::new(fposmod(self.x, pmod), fposmod(self.y, pmod))
    }

    pub fn posmodv(self, modv: Self) -> Self {
        Self::new(fposmod(self.x, modv.x), fposmod(self.y, modv.y))
    }

    pub fn project(self, b: Self) -> Self {
        Self::from_glam(self.to_glam().project_onto(b.to_glam()))
    }

    pub fn reflect(self, normal: Self) -> Self {
        Self::from_glam(self.to_glam().reject_from(normal.to_glam()))
    }

    pub fn round(self) -> Self {
        Self::from_glam(self.to_glam().round())
    }

    pub fn sign(self) -> Self {
        Self::new(sign(self.x), sign(self.y))
    }

    // TODO compare with gdnative implementation:
    // https://github.com/godot-rust/gdnative/blob/master/gdnative-core/src/core_types/vector3.rs#L335-L343
    pub fn slerp(self, to: Self, weight: real) -> Self {
        let start_length_sq = self.length_squared();
        let end_length_sq = to.length_squared();
        if start_length_sq == 0.0 || end_length_sq == 0.0 {
            return self.lerp(to, weight);
        }
        let start_length = start_length_sq.sqrt();
        let result_length = lerp(start_length, end_length_sq.sqrt(), weight);
        let angle = self.angle_to(to);
        self.rotated(angle * weight) * (result_length / start_length)
    }

    pub fn slide(self, normal: Self) -> Self {
        self - normal * self.dot(normal)
    }

    pub fn snapped(self, step: Self) -> Self {
        Self::new(snapped(self.x, step.x), snapped(self.y, step.y))
    }

    /// Returns the result of rotating this vector by `angle` (in radians).
    pub fn rotated(self, angle: real) -> Self {
        Self::from_glam(RAffine2::from_angle(angle).transform_vector2(self.to_glam()))
    }

    #[doc(hidden)]
    pub fn as_inner(&self) -> inner::InnerVector2 {
        inner::InnerVector2::from_outer(self)
    }
}

/// Formats the vector like Godot: `(x, y)`.
impl fmt::Display for Vector2 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

impl_common_vector_fns!(Vector2, real);
impl_float_vector_fns!(Vector2, real);
impl_vector_operators!(Vector2, real, (x, y));
impl_vector_index!(Vector2, real, (x, y), Vector2Axis, (X, Y));

// SAFETY:
// This type is represented as `Self` in Godot, so `*mut Self` is sound.
unsafe impl GodotFfi for Vector2 {
    ffi_methods! { type sys::GDExtensionTypePtr = *mut Self; .. }
}

/// Enumerates the axes in a [`Vector2`].
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
#[repr(i32)]
pub enum Vector2Axis {
    /// The X axis.
    X,

    /// The Y axis.
    Y,
}

// SAFETY:
// This type is represented as `Self` in Godot, so `*mut Self` is sound.
unsafe impl GodotFfi for Vector2Axis {
    ffi_methods! { type sys::GDExtensionTypePtr = *mut Self; .. }
}

impl GlamType for RVec2 {
    type Mapped = Vector2;

    fn to_front(&self) -> Self::Mapped {
        Vector2::new(self.x, self.y)
    }

    fn from_front(mapped: &Self::Mapped) -> Self {
        RVec2::new(mapped.x, mapped.y)
    }
}

impl GlamConv for Vector2 {
    type Glam = RVec2;
}

#[cfg(test)]
mod test {
    use crate::assert_eq_approx;

    use super::*;

    #[test]
    fn coord_min_max() {
        let a = Vector2::new(1.2, 3.4);
        let b = Vector2::new(0.1, 5.6);
        assert_eq_approx!(
            a.coord_min(b),
            Vector2::new(0.1, 3.4),
            Vector2::is_equal_approx
        );
        assert_eq_approx!(
            a.coord_max(b),
            Vector2::new(1.2, 5.6),
            Vector2::is_equal_approx
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_roundtrip() {
        let vector = Vector2::default();
        let expected_json = "{\"x\":0.0,\"y\":0.0}";

        crate::builtin::test_utils::roundtrip(&vector, expected_json);
    }
}
