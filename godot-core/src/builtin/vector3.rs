/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
use std::ops::*;

use std::fmt;

use glam::f32::Vec3;
use glam::Vec3A;
use godot_ffi as sys;
use sys::{ffi_methods, GodotFfi};

use crate::builtin::math::*;
use crate::builtin::Vector3i;

use super::glam_helpers::GlamConv;
use super::glam_helpers::GlamType;

/// Vector used for 3D math using floating point coordinates.
///
/// 3-element structure that can be used to represent positions in 3D space or any other triple of
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

    /// Unit vector in +Y direction. Typically interpreted as up in a 3D world.
    pub const UP: Self = Self::new(0.0, 1.0, 0.0);

    /// Unit vector in -Y direction. Typically interpreted as down in a 3D world.
    pub const DOWN: Self = Self::new(0.0, -1.0, 0.0);

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

    pub fn angle_to(self, to: Self) -> f32 {
        self.to_glam().angle_between(to.to_glam())
    }

    pub fn bezier_derivative(self, control_1: Self, control_2: Self, end: Self, t: f32) -> Self {
        let x = bezier_derivative(self.x, control_1.x, control_2.x, end.x, t);
        let y = bezier_derivative(self.y, control_1.y, control_2.y, end.y, t);
        let z = bezier_derivative(self.z, control_1.z, control_2.z, end.z, t);

        Self::new(x, y, z)
    }

    pub fn bezier_interpolate(self, control_1: Self, control_2: Self, end: Self, t: f32) -> Self {
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

    pub fn cubic_interpolate(self, b: Self, pre_a: Self, post_b: Self, weight: f32) -> Self {
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
        weight: f32,
        b_t: f32,
        pre_a_t: f32,
        post_b_t: f32,
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

    pub fn distance_squared_to(self, to: Self) -> f32 {
        (to - self).length_squared()
    }

    pub fn distance_to(self, to: Self) -> f32 {
        (to - self).length()
    }

    pub fn dot(self, with: Self) -> f32 {
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

    pub fn length_squared(self) -> f32 {
        self.to_glam().length_squared()
    }

    pub fn lerp(self, to: Self, weight: f32) -> Self {
        Self::from_glam(self.to_glam().lerp(to.to_glam(), weight))
    }

    pub fn limit_length(self, length: Option<f32>) -> Self {
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

    pub fn move_toward(self, to: Self, delta: f32) -> Self {
        let vd = to - self;
        let len = vd.length();
        if len <= delta || len < CMP_EPSILON {
            to
        } else {
            self + vd / len * delta
        }
    }

    pub fn posmod(self, pmod: f32) -> Self {
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

    pub fn signed_angle_to(self, to: Self, axis: Self) -> f32 {
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

impl GlamType for Vec3 {
    type Mapped = Vector3;

    fn to_front(&self) -> Self::Mapped {
        Vector3::new(self.x, self.y, self.z)
    }

    fn from_front(mapped: &Self::Mapped) -> Self {
        Vec3::new(mapped.x, mapped.y, mapped.z)
    }
}

impl GlamType for Vec3A {
    type Mapped = Vector3;

    fn to_front(&self) -> Self::Mapped {
        Vector3::new(self.x, self.y, self.z)
    }

    fn from_front(mapped: &Self::Mapped) -> Self {
        Vec3A::new(mapped.x, mapped.y, mapped.z)
    }
}

impl GlamConv for Vector3 {
    type Glam = Vec3;
}
