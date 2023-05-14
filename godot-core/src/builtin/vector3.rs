/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
use std::ops::*;

use std::fmt;

use godot_ffi as sys;
use sys::{ffi_methods, GodotFfi};

use crate::builtin::math::*;
use crate::builtin::Vector3i;

use super::glam_helpers::GlamConv;
use super::glam_helpers::GlamType;
use super::{real, Basis, RVec3};

/// Vector used for 3D math using floating point coordinates.
///
/// 3-element structure that can be used to represent positions in 3D space or any other triple of
/// numeric values.
///
/// It uses floating-point coordinates of 32-bit precision, unlike the engine's `float` type which
/// is always 64-bit. The engine can be compiled with the option `precision=double` to use 64-bit
/// vectors; use the gdext library with the `double-precision` feature in that case.
///
/// See [`Vector3i`] for its integer counterpart.
#[derive(Default, Copy, Clone, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(C)]
pub struct Vector3 {
    /// The vector's X component.
    pub x: real,

    /// The vector's Y component.
    pub y: real,

    /// The vector's Z component.
    pub z: real,
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

    /// Unit vector in +Y direction. Typically interpreted as up in a 3D world.
    pub const UP: Self = Self::new(0.0, 1.0, 0.0);

    /// Unit vector in -Y direction. Typically interpreted as down in a 3D world.
    pub const DOWN: Self = Self::new(0.0, -1.0, 0.0);

    /// Unit vector in -Z direction. Can be interpreted as "into the screen" in an untransformed 3D world.
    pub const FORWARD: Self = Self::new(0.0, 0.0, -1.0);

    /// Unit vector in +Z direction. Can be interpreted as "out of the screen" in an untransformed 3D world.
    pub const BACK: Self = Self::new(0.0, 0.0, 1.0);

    /// Returns a `Vector3` with the given components.
    pub const fn new(x: real, y: real, z: real) -> Self {
        Self { x, y, z }
    }

    /// Returns a new `Vector3` with all components set to `v`.
    pub const fn splat(v: real) -> Self {
        Self::new(v, v, v)
    }

    /// Constructs a new `Vector3` from a [`Vector3i`].
    pub const fn from_vector3i(v: Vector3i) -> Self {
        Self {
            x: v.x as real,
            y: v.y as real,
            z: v.z as real,
        }
    }

    /// Converts the corresponding `glam` type to `Self`.
    fn from_glam(v: RVec3) -> Self {
        Self::new(v.x, v.y, v.z)
    }

    /// Converts `self` to the corresponding `glam` type.
    fn to_glam(self) -> RVec3 {
        RVec3::new(self.x, self.y, self.z)
    }

    pub fn angle_to(self, to: Self) -> real {
        self.to_glam().angle_between(to.to_glam())
    }

    pub fn bezier_derivative(self, control_1: Self, control_2: Self, end: Self, t: real) -> Self {
        let x = bezier_derivative(self.x, control_1.x, control_2.x, end.x, t);
        let y = bezier_derivative(self.y, control_1.y, control_2.y, end.y, t);
        let z = bezier_derivative(self.z, control_1.z, control_2.z, end.z, t);

        Self::new(x, y, z)
    }

    pub fn bezier_interpolate(self, control_1: Self, control_2: Self, end: Self, t: real) -> Self {
        let x = bezier_interpolate(self.x, control_1.x, control_2.x, end.x, t);
        let y = bezier_interpolate(self.y, control_1.y, control_2.y, end.y, t);
        let z = bezier_interpolate(self.z, control_1.z, control_2.z, end.z, t);

        Self::new(x, y, z)
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

    pub fn cross(self, with: Self) -> Self {
        Self::from_glam(self.to_glam().cross(with.to_glam()))
    }

    pub fn cubic_interpolate(self, b: Self, pre_a: Self, post_b: Self, weight: real) -> Self {
        let x = cubic_interpolate(self.x, b.x, pre_a.x, post_b.x, weight);
        let y = cubic_interpolate(self.y, b.y, pre_a.y, post_b.y, weight);
        let z = cubic_interpolate(self.z, b.z, pre_a.z, post_b.z, weight);

        Self::new(x, y, z)
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
        let z = cubic_interpolate_in_time(
            self.z, b.z, pre_a.z, post_b.z, weight, b_t, pre_a_t, post_b_t,
        );

        Self::new(x, y, z)
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

    pub fn dot(self, with: Self) -> real {
        self.to_glam().dot(with.to_glam())
    }

    pub fn floor(self) -> Self {
        Self::from_glam(self.to_glam().floor())
    }

    pub fn inverse(self) -> Self {
        Self::new(1.0 / self.x, 1.0 / self.y, 1.0 / self.z)
    }

    pub fn is_equal_approx(self, to: Self) -> bool {
        is_equal_approx(self.x, to.x)
            && is_equal_approx(self.y, to.y)
            && is_equal_approx(self.z, to.z)
    }

    pub fn is_finite(self) -> bool {
        self.to_glam().is_finite()
    }

    pub fn is_normalized(self) -> bool {
        self.to_glam().is_normalized()
    }

    pub fn is_zero_approx(self) -> bool {
        is_zero_approx(self.x) && is_zero_approx(self.y) && is_zero_approx(self.z)
    }

    pub fn length_squared(self) -> real {
        self.to_glam().length_squared()
    }

    pub fn lerp(self, to: Self, weight: real) -> Self {
        Self::from_glam(self.to_glam().lerp(to.to_glam(), weight))
    }

    pub fn limit_length(self, length: Option<real>) -> Self {
        Self::from_glam(self.to_glam().clamp_length_max(length.unwrap_or(1.0)))
    }

    pub fn max_axis_index(self) -> Vector3Axis {
        if self.x < self.y {
            if self.y < self.z {
                Vector3Axis::Z
            } else {
                Vector3Axis::Y
            }
        } else if self.x < self.z {
            Vector3Axis::Z
        } else {
            Vector3Axis::X
        }
    }

    pub fn min_axis_index(self) -> Vector3Axis {
        if self.x < self.y {
            if self.x < self.z {
                Vector3Axis::X
            } else {
                Vector3Axis::Z
            }
        } else if self.y < self.z {
            Vector3Axis::Y
        } else {
            Vector3Axis::Z
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

    pub fn posmod(self, pmod: real) -> Self {
        Self::new(
            fposmod(self.x, pmod),
            fposmod(self.y, pmod),
            fposmod(self.z, pmod),
        )
    }

    pub fn posmodv(self, modv: Self) -> Self {
        Self::new(
            fposmod(self.x, modv.x),
            fposmod(self.y, modv.y),
            fposmod(self.z, modv.z),
        )
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
        Self::new(sign(self.x), sign(self.y), sign(self.z))
    }

    pub fn signed_angle_to(self, to: Self, axis: Self) -> real {
        let cross_to = self.cross(to);
        let unsigned_angle = self.dot(to).atan2(cross_to.length());
        let sign = cross_to.dot(axis);
        if sign < 0.0 {
            -unsigned_angle
        } else {
            unsigned_angle
        }
    }

    pub fn slide(self, normal: Self) -> Self {
        self - normal * self.dot(normal)
    }

    pub fn snapped(self, step: Self) -> Self {
        Self::new(
            snapped(self.x, step.x),
            snapped(self.y, step.y),
            snapped(self.z, step.z),
        )
    }

    /// Returns this vector rotated around `axis` by `angle` radians. `axis` must be normalized.
    ///
    /// # Panics
    /// If `axis` is not normalized.
    pub fn rotated(self, axis: Self, angle: real) -> Self {
        assert!(axis.is_normalized());
        Basis::from_axis_angle(axis, angle) * self
    }
}

/// Formats the vector like Godot: `(x, y, z)`.
impl fmt::Display for Vector3 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {}, {})", self.x, self.y, self.z)
    }
}

impl_common_vector_fns!(Vector3, real);
impl_float_vector_fns!(Vector3, real);
impl_vector_operators!(Vector3, real, (x, y, z));
impl_vector_index!(Vector3, real, (x, y, z), Vector3Axis, (X, Y, Z));

// SAFETY:
// This type is represented as `Self` in Godot, so `*mut Self` is sound.
unsafe impl GodotFfi for Vector3 {
    ffi_methods! { type sys::GDExtensionTypePtr = *mut Self; .. }
}

/// Enumerates the axes in a [`Vector3`].
// TODO auto-generate this, alongside all the other builtin type's enums
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
#[repr(i32)]
pub enum Vector3Axis {
    /// The X axis.
    X,

    /// The Y axis.
    Y,

    /// The Z axis.
    Z,
}

// SAFETY:
// This type is represented as `Self` in Godot, so `*mut Self` is sound.
unsafe impl GodotFfi for Vector3Axis {
    ffi_methods! { type sys::GDExtensionTypePtr = *mut Self; .. }
}

impl GlamType for RVec3 {
    type Mapped = Vector3;

    fn to_front(&self) -> Self::Mapped {
        Vector3::new(self.x, self.y, self.z)
    }

    fn from_front(mapped: &Self::Mapped) -> Self {
        RVec3::new(mapped.x, mapped.y, mapped.z)
    }
}

#[cfg(not(feature = "double-precision"))]
impl GlamType for glam::Vec3A {
    type Mapped = Vector3;

    fn to_front(&self) -> Self::Mapped {
        Vector3::new(self.x, self.y, self.z)
    }

    fn from_front(mapped: &Self::Mapped) -> Self {
        glam::Vec3A::new(mapped.x, mapped.y, mapped.z)
    }
}

impl GlamConv for Vector3 {
    type Glam = RVec3;
}

#[cfg(test)]
mod test {
    use crate::assert_eq_approx;

    use super::*;
    use godot::builtin::real_consts::TAU;

    // Translated from Godot
    #[test]
    #[allow(clippy::excessive_precision)]
    fn rotation() {
        let vector = Vector3::new(1.2, 3.4, 5.6);
        assert_eq_approx!(
            vector.rotated(Vector3::new(0.0, 1.0, 0.0), TAU),
            vector,
            Vector3::is_equal_approx
        );
        assert_eq_approx!(
            vector.rotated(Vector3::new(0.0, 1.0, 0.0), TAU / 4.0),
            Vector3::new(5.6, 3.4, -1.2),
            Vector3::is_equal_approx
        );
        assert_eq_approx!(
            vector.rotated(Vector3::new(1.0, 0.0, 0.0), TAU / 3.0),
            Vector3::new(1.2, -6.54974226119285642, 0.1444863728670914),
            Vector3::is_equal_approx
        );
        assert_eq_approx!(
            vector.rotated(Vector3::new(0.0, 0.0, 1.0), TAU / 2.0),
            vector.rotated(Vector3::new(0.0, 0.0, 1.0), TAU / -2.0),
            Vector3::is_equal_approx
        );
    }

    #[test]
    fn coord_min_max() {
        let a = Vector3::new(1.2, 3.4, 5.6);
        let b = Vector3::new(0.1, 5.6, 2.3);
        assert_eq_approx!(
            a.coord_min(b),
            Vector3::new(0.1, 3.4, 2.3),
            Vector3::is_equal_approx
        );
        assert_eq_approx!(
            a.coord_max(b),
            Vector3::new(1.2, 5.6, 5.6),
            Vector3::is_equal_approx
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_roundtrip() {
        let vector = Vector3::default();
        let expected_json = "{\"x\":0.0,\"y\":0.0,\"z\":0.0}";

        crate::builtin::test_utils::roundtrip(&vector, expected_json);
    }
}
