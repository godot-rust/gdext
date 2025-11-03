/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

use godot_ffi as sys;
use sys::{ffi_methods, ExtVariantType, GodotFfi};

use crate::builtin::math::{ApproxEq, FloatExt, GlamConv, GlamType};
use crate::builtin::{inner, real, Basis, EulerOrder, RQuat, RealConv, Vector3};

/// Unit quaternion to represent 3D rotations.
///
/// See also [`Quaternion`](https://docs.godotengine.org/en/stable/classes/class_quaternion.html) in the Godot documentation.
///
/// # Godot docs
///
/// [`Quaternion` (stable)](https://docs.godotengine.org/en/stable/classes/class_quaternion.html)
#[derive(Copy, Clone, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(C)]
pub struct Quaternion {
    pub x: real,
    pub y: real,
    pub z: real,
    pub w: real,
}

impl Quaternion {
    /// The identity quaternion, representing no rotation. This has the same rotation as [`Basis::IDENTITY`].
    ///
    /// If a [`Vector3`] is rotated (multiplied) by this quaternion, it does not change.
    pub const IDENTITY: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
        w: 1.0,
    };

    pub fn new(x: real, y: real, z: real, w: real) -> Self {
        Self { x, y, z, w }
    }

    /// Creates a quaternion from a Vector3 and an angle.
    ///
    /// # Panics
    /// If the vector3 is not normalized.
    pub fn from_axis_angle(axis: Vector3, angle: real) -> Self {
        assert!(
            axis.is_normalized(),
            "Quaternion axis {axis:?} is not normalized."
        );
        let d = axis.length();
        let sin_angle = (angle * 0.5).sin();
        let cos_angle = (angle * 0.5).cos();
        let s = sin_angle / d;
        let x = axis.x * s;
        let y = axis.y * s;
        let z = axis.z * s;
        let w = cos_angle;
        Self::new(x, y, z, w)
    }

    /// Constructs a Quaternion representing the shortest arc between `arc_from` and `arc_to`.
    ///
    /// These can be imagined as two points intersecting a unit sphere's surface, with a radius of 1.0.
    ///
    // This is an undocumented assumption of Godot's as well.
    /// The inputs must be unit vectors.
    ///
    /// For near-singular cases (`arc_from`≈`arc_to` or `arc_from`≈-`arc_to`) the current implementation is only accurate to about
    /// 0.001, or better if `double-precision` is enabled.
    ///
    /// *Godot equivalent: `Quaternion(arc_from: Vector3, arc_to: Vector3)`*
    pub fn from_rotation_arc(arc_from: Vector3, arc_to: Vector3) -> Self {
        sys::balanced_assert!(
            arc_from.is_normalized(),
            "input 1 (`arc_from`) in `Quaternion::from_rotation_arc` must be a unit vector"
        );
        sys::balanced_assert!(
            arc_to.is_normalized(),
            "input 2 (`arc_to`) in `Quaternion::from_rotation_arc` must be a unit vector"
        );
        <Self as GlamConv>::Glam::from_rotation_arc(arc_from.to_glam(), arc_to.to_glam()).to_front()
    }

    pub fn angle_to(self, to: Self) -> real {
        self.glam2(&to, RQuat::angle_between)
    }

    pub fn dot(self, with: Self) -> real {
        self.glam2(&with, RQuat::dot)
    }

    pub fn exp(self) -> Self {
        let mut v = Vector3::new(self.x, self.y, self.z);
        let theta = v.length();
        v = v.normalized();

        if theta < real::CMP_EPSILON || !v.is_normalized() {
            Self::default()
        } else {
            Self::from_axis_angle(v, theta)
        }
    }

    pub fn from_euler(euler: Vector3) -> Self {
        let half_a1 = euler.y * 0.5;
        let half_a2 = euler.x * 0.5;
        let half_a3 = euler.z * 0.5;
        let cos_a1 = half_a1.cos();
        let sin_a1 = half_a1.sin();
        let cos_a2 = half_a2.cos();
        let sin_a2 = half_a2.sin();
        let cos_a3 = half_a3.cos();
        let sin_a3 = half_a3.sin();

        Self::new(
            sin_a1 * cos_a2 * sin_a3 + cos_a1 * sin_a2 * cos_a3,
            sin_a1 * cos_a2 * cos_a3 - cos_a1 * sin_a2 * sin_a3,
            -sin_a1 * sin_a2 * cos_a3 + cos_a1 * cos_a2 * sin_a3,
            sin_a1 * sin_a2 * sin_a3 + cos_a1 * cos_a2 * cos_a3,
        )
    }

    pub fn get_angle(self) -> real {
        2.0 * self.w.acos()
    }

    pub fn get_axis(self) -> Vector3 {
        let Self { x, y, z, w } = self;
        let axis = Vector3::new(x, y, z);

        if self.w.abs() > 1.0 - real::CMP_EPSILON {
            axis
        } else {
            let r = 1.0 / (1.0 - w * w).sqrt();
            r * axis
        }
    }

    /// Returns the rotation of the matrix in euler angles, with the order `YXZ`.
    ///
    /// See [`get_euler_with()`](Self::get_euler_with) for custom angle orders.
    pub fn get_euler(self) -> Vector3 {
        self.get_euler_with(EulerOrder::YXZ)
    }

    /// Returns the rotation of the matrix in euler angles.
    ///
    /// The order of the angles are given by `order`. To use the default order `YXZ`, see [`get_euler()`](Self::get_euler).
    ///
    /// _Godot equivalent: `Quaternion.get_euler()`_
    pub fn get_euler_with(self, order: EulerOrder) -> Vector3 {
        Basis::from_quaternion(self).get_euler_with(order)
    }

    pub fn inverse(self) -> Self {
        Self::new(-self.x, -self.y, -self.z, self.w)
    }

    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite() && self.w.is_finite()
    }

    pub fn is_normalized(self) -> bool {
        self.length_squared().approx_eq(&1.0)
    }

    pub fn length(self) -> real {
        self.length_squared().sqrt()
    }

    pub fn length_squared(self) -> real {
        self.dot(self)
    }

    pub fn log(self) -> Self {
        let v = self.get_axis() * self.get_angle();
        Quaternion::new(v.x, v.y, v.z, 0.0)
    }

    /// # Panics
    /// If the quaternion has length of 0.
    pub fn normalized(self) -> Self {
        let length = self.length();
        assert!(!length.approx_eq(&0.0), "Quaternion has length 0");
        self / length
    }

    /// # Panics
    /// If either quaternion is not normalized.
    pub fn slerp(self, to: Self, weight: real) -> Self {
        let normalized_inputs = self.ensure_normalized(&[&to]);
        assert!(normalized_inputs, "Slerp requires normalized quaternions");

        self.as_inner().slerp(to, weight.as_f64())
    }

    /// # Panics
    /// If either quaternion is not normalized.
    pub fn slerpni(self, to: Self, weight: real) -> Self {
        let normalized_inputs = self.ensure_normalized(&[&to]);
        assert!(normalized_inputs, "Slerpni requires normalized quaternions");

        self.as_inner().slerpni(to, weight.as_f64())
    }

    /// # Panics
    /// If any quaternions are not normalized.
    pub fn spherical_cubic_interpolate(
        self,
        b: Self,
        pre_a: Self,
        post_b: Self,
        weight: real,
    ) -> Self {
        let normalized_inputs = self.ensure_normalized(&[&b, &pre_a, &post_b]);
        assert!(
            normalized_inputs,
            "Spherical cubic interpolation requires normalized quaternions"
        );

        self.as_inner()
            .spherical_cubic_interpolate(b, pre_a, post_b, weight.as_f64())
    }

    /// # Panics
    /// If any quaternions are not normalized.
    #[allow(clippy::too_many_arguments)]
    pub fn spherical_cubic_interpolate_in_time(
        self,
        b: Self,
        pre_a: Self,
        post_b: Self,
        weight: real,
        b_t: real,
        pre_a_t: real,
        post_b_t: real,
    ) -> Self {
        let normalized_inputs = self.ensure_normalized(&[&b, &pre_a, &post_b]);
        assert!(
            normalized_inputs,
            "Spherical cubic interpolation in time requires normalized quaternions"
        );

        self.as_inner().spherical_cubic_interpolate_in_time(
            b,
            pre_a,
            post_b,
            weight.as_f64(),
            b_t.as_f64(),
            pre_a_t.as_f64(),
            post_b_t.as_f64(),
        )
    }

    #[doc(hidden)]
    pub fn as_inner(&self) -> inner::InnerQuaternion<'_> {
        inner::InnerQuaternion::from_outer(self)
    }

    #[doc(hidden)]
    fn ensure_normalized(&self, quats: &[&Quaternion]) -> bool {
        quats.iter().all(|v| v.is_normalized()) && self.is_normalized()
    }
}

impl Add for Quaternion {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self::new(
            self.x + other.x,
            self.y + other.y,
            self.z + other.z,
            self.w + other.w,
        )
    }
}

impl AddAssign for Quaternion {
    fn add_assign(&mut self, other: Self) {
        *self = *self + other
    }
}

impl Sub for Quaternion {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self::new(
            self.x - other.x,
            self.y - other.y,
            self.z - other.z,
            self.w - other.w,
        )
    }
}

impl SubAssign for Quaternion {
    fn sub_assign(&mut self, other: Self) {
        *self = *self - other
    }
}

impl Mul<Quaternion> for Quaternion {
    type Output = Self;

    fn mul(self, other: Quaternion) -> Self {
        // TODO use super::glam?

        let x = self.w * other.x + self.x * other.w + self.y * other.z - self.z * other.y;
        let y = self.w * other.y + self.y * other.w + self.z * other.x - self.x * other.z;
        let z = self.w * other.z + self.z * other.w + self.x * other.y - self.y * other.x;
        let w = self.w * other.w - self.x * other.x - self.y * other.y - self.z * other.z;

        Self::new(x, y, z, w)
    }
}

impl Mul<Vector3> for Quaternion {
    type Output = Vector3;

    /// Applies the quaternion's rotation to the 3D point represented by the vector.
    ///
    /// # Panics
    /// If the quaternion is not normalized.
    fn mul(self, rhs: Vector3) -> Self::Output {
        Vector3::from_glam(self.to_glam().mul_vec3(rhs.to_glam()))
    }
}

// SAFETY:
// This type is represented as `Self` in Godot, so `*mut Self` is sound.
unsafe impl GodotFfi for Quaternion {
    const VARIANT_TYPE: ExtVariantType = ExtVariantType::Concrete(sys::VariantType::QUATERNION);

    ffi_methods! { type sys::GDExtensionTypePtr = *mut Self; .. }
}

crate::meta::impl_godot_as_self!(Quaternion: ByValue);

impl std::fmt::Display for Quaternion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.to_glam().fmt(f)
    }
}

impl Default for Quaternion {
    fn default() -> Self {
        Self::new(0.0, 0.0, 0.0, 1.0)
    }
}

impl ApproxEq for Quaternion {
    fn approx_eq(&self, other: &Self) -> bool {
        self.x.approx_eq(&other.x)
            && self.y.approx_eq(&other.y)
            && self.z.approx_eq(&other.z)
            && self.w.approx_eq(&other.w)
    }
}

impl GlamConv for Quaternion {
    type Glam = RQuat;
}

impl GlamType for RQuat {
    type Mapped = Quaternion;

    fn to_front(&self) -> Self::Mapped {
        Quaternion::new(self.x, self.y, self.z, self.w)
    }

    fn from_front(mapped: &Self::Mapped) -> Self {
        RQuat::from_xyzw(mapped.x, mapped.y, mapped.z, mapped.w)
    }
}

impl MulAssign<Quaternion> for Quaternion {
    fn mul_assign(&mut self, other: Quaternion) {
        *self = *self * other
    }
}

impl Mul<real> for Quaternion {
    type Output = Self;

    fn mul(self, other: real) -> Self {
        Quaternion::new(
            self.x * other,
            self.y * other,
            self.z * other,
            self.w * other,
        )
    }
}

impl Mul<Quaternion> for real {
    type Output = Quaternion;

    fn mul(self, other: Quaternion) -> Quaternion {
        other * self
    }
}

impl MulAssign<real> for Quaternion {
    fn mul_assign(&mut self, other: real) {
        *self = *self * other
    }
}

impl Div<real> for Quaternion {
    type Output = Self;

    fn div(self, other: real) -> Self {
        Self::new(
            self.x / other,
            self.y / other,
            self.z / other,
            self.w / other,
        )
    }
}

impl DivAssign<real> for Quaternion {
    fn div_assign(&mut self, other: real) {
        *self = *self / other
    }
}

impl Neg for Quaternion {
    type Output = Self;

    fn neg(self) -> Self {
        Self::new(-self.x, -self.y, -self.z, -self.w)
    }
}

#[cfg(test)]
mod test {
    #[cfg(feature = "serde")]
    #[test]
    fn serde_roundtrip() {
        let quaternion = super::Quaternion::new(1.0, 1.0, 1.0, 1.0);
        let expected_json = "{\"x\":1.0,\"y\":1.0,\"z\":1.0,\"w\":1.0}";

        crate::builtin::test_utils::roundtrip(&quaternion, expected_json);
    }
}
