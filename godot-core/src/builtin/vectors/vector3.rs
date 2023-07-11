/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot_ffi as sys;
use sys::{ffi_methods, GodotFfi};

use crate::builtin::math::{FloatExt, GlamConv, GlamType};
use crate::builtin::vectors::Vector3Axis;
use crate::builtin::{real, Basis, RVec3, Vector3i};

use std::fmt;

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

    pub fn is_finite(self) -> bool {
        self.to_glam().is_finite()
    }

    pub fn is_normalized(self) -> bool {
        self.to_glam().is_normalized()
    }

    pub fn length_squared(self) -> real {
        self.to_glam().length_squared()
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
        if len <= delta || len < real::CMP_EPSILON {
            to
        } else {
            self + vd / len * delta
        }
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

    /// Returns the spherical linear interpolation between the vector and `to` by the `weight` amount.
    ///
    /// The variable `weight` is representing the amount of interpolation, which is on the range of
    /// 0.0 to 1.0.
    ///
    /// Length is also interpolated in the case that the input vectors have different lengths. If both
    /// input vectors have zero length or are collinear to each other, the method instead behaves like
    /// [`Vector3::lerp`].
    pub fn slerp(self, to: Self, weight: real) -> Self {
        let start_length_sq: real = self.length_squared();
        let end_length_sq = to.length_squared();
        if start_length_sq == 0.0 || end_length_sq == 0.0 {
            // Vectors with zero length do not have an angle relative to the origin point, so it cannot
            // produce a cross product for determining the angle to slerp into. Because of this, lerp
            // is used to interpolate between the two vectors.
            return self.lerp(to, weight);
        }

        let axis = self.cross(to);
        if axis == Vector3::ZERO {
            // Two collinear vectors do not have a unique perpendicular axis to both of them, so it
            // cannot produce a cross product for determining the angle to slerp into. Because of this,
            // lerp is used to interpolate between the two vectors.
            return self.lerp(to, weight);
        }

        let unit_axis = axis.normalized();
        let start_length = start_length_sq.sqrt();
        let result_length = start_length.lerp(end_length_sq.sqrt(), weight);
        let angle = self.angle_to(to);
        self.rotated(unit_axis, angle * weight) * (result_length / start_length)
    }

    pub fn slide(self, normal: Self) -> Self {
        self - normal * self.dot(normal)
    }

    /// Returns this vector rotated around `axis` by `angle` radians. `axis` must be normalized.
    ///
    /// # Panics
    /// If `axis` is not normalized.
    pub fn rotated(self, axis: Self, angle: real) -> Self {
        assert!(axis.is_normalized());
        Basis::from_axis_angle(axis, angle) * self
    }

    pub fn coords(&self) -> (real, real, real) {
        (self.x, self.y, self.z)
    }
}

/// Formats the vector like Godot: `(x, y, z)`.
impl fmt::Display for Vector3 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {}, {})", self.x, self.y, self.z)
    }
}

impl_common_vector_fns!(Vector3, real);
impl_float_vector_glam_fns!(Vector3, real);
impl_float_vector_component_fns!(Vector3, real, (x, y, z));
impl_vector_operators!(Vector3, real, (x, y, z));
impl_from_tuple_for_vector3x!(Vector3, real);

// SAFETY:
// This type is represented as `Self` in Godot, so `*mut Self` is sound.
unsafe impl GodotFfi for Vector3 {
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
    use super::*;
    use crate::builtin::math::assert_eq_approx;
    use crate::builtin::real_consts::{SQRT_2, TAU};

    // Translated from Godot
    #[test]
    #[allow(clippy::excessive_precision)]
    fn rotation() {
        let vector = Vector3::new(1.2, 3.4, 5.6);
        assert_eq_approx!(
            vector.rotated(Vector3::new(0.0, 1.0, 0.0), TAU), //.
            vector
        );
        assert_eq_approx!(
            vector.rotated(Vector3::new(0.0, 1.0, 0.0), TAU / 4.0),
            Vector3::new(5.6, 3.4, -1.2),
        );
        assert_eq_approx!(
            vector.rotated(Vector3::new(1.0, 0.0, 0.0), TAU / 3.0),
            Vector3::new(1.2, -6.54974226119285642, 0.1444863728670914),
        );
        assert_eq_approx!(
            vector.rotated(Vector3::new(0.0, 0.0, 1.0), TAU / 2.0),
            vector.rotated(Vector3::new(0.0, 0.0, 1.0), TAU / -2.0),
        );
    }

    #[test]
    fn coord_min_max() {
        let a = Vector3::new(1.2, 3.4, 5.6);
        let b = Vector3::new(0.1, 5.6, 2.3);

        assert_eq_approx!(a.coord_min(b), Vector3::new(0.1, 3.4, 2.3));
        assert_eq_approx!(a.coord_max(b), Vector3::new(1.2, 5.6, 5.6));
    }

    #[test]
    fn test_slerp() {
        // The halfway point of a slerp operation on two vectors on a circle is the halfway point of
        // the arc length between the two vectors.
        let vector_from = Vector3::new(0.0, 2.0, 0.0);
        let vector_to = Vector3::new(2.0, 0.0, 0.0);
        let vector_in_between = Vector3::new(SQRT_2, SQRT_2, 0.0);
        assert_eq_approx!(vector_from.slerp(vector_to, 0.5), vector_in_between);

        // Collinear vectors cannot be slerped so the halfway point of the slerp operation on them is
        // just the halfway point between them.
        let vector_from = Vector3::new(0.0, 2.0, 0.0);
        let vector_to = Vector3::new(0.0, -2.0, 0.0);
        assert_eq_approx!(vector_from.slerp(vector_to, 0.5), Vector3::ZERO);

        let vector_from = Vector3::new(0.0, 3.0, 0.0);
        let vector_to = Vector3::new(0.0, 2.0, 0.0);
        assert_eq_approx!(
            vector_from.slerp(vector_to, 0.5),
            Vector3::new(0.0, 2.5, 0.0)
        );

        // Ported Godot slerp tests.
        let vector1 = Vector3::new(1.0, 2.0, 3.0);
        let vector2 = Vector3::new(4.0, 5.0, 6.0);
        assert_eq_approx!(
            vector1.normalized().slerp(vector2.normalized(), 0.5),
            Vector3::new(0.363_866_8, 0.555_698_2, 0.747_529_57)
        );
        assert_eq_approx!(
            vector1.normalized().slerp(vector2.normalized(), 1.0 / 3.0),
            Vector3::new(0.332_119_76, 0.549_413_74, 0.766_707_84)
        );
        assert_eq_approx!(
            Vector3::new(5.0, 0.0, 0.0).slerp(Vector3::new(0.0, 3.0, 4.0), 0.5),
            Vector3::new(3.535_534, 2.121_320_5, 2.828_427_3)
        );
        assert_eq_approx!(
            Vector3::new(1.0, 1.0, 1.0).slerp(Vector3::new(2.0, 2.0, 2.0), 0.5),
            Vector3::new(1.5, 1.5, 1.5)
        );
        assert_eq!(Vector3::ZERO.slerp(Vector3::ZERO, 0.5), Vector3::ZERO);
        assert_eq!(
            Vector3::ZERO.slerp(Vector3::new(1.0, 1.0, 1.0), 0.5),
            Vector3::new(0.5, 0.5, 0.5)
        );
        assert_eq!(
            Vector3::new(1.0, 1.0, 1.0).slerp(Vector3::ZERO, 0.5),
            Vector3::new(0.5, 0.5, 0.5)
        );
        assert_eq_approx!(
            Vector3::new(4.0, 6.0, 2.0).slerp(Vector3::new(8.0, 10.0, 3.0), 0.5),
            Vector3::new(5.901_942_3, 8.067_587, 2.558_308)
        );
        assert_eq_approx!(vector1.slerp(vector2, 0.5).length(), real!(6.258_311));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_roundtrip() {
        let vector = Vector3::default();
        let expected_json = "{\"x\":0.0,\"y\":0.0,\"z\":0.0}";

        crate::builtin::test_utils::roundtrip(&vector, expected_json);
    }
}
