/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
use std::ops::*;

use godot_ffi as sys;
use sys::{ffi_methods, GodotFfi};

use crate::builtin::{math::*, vector3::*};

type Inner = glam::f32::Quat;

#[derive(Default, Copy, Clone, Debug, PartialEq)]
#[repr(C)]
pub struct Quaternion {
    inner: Inner,
}

impl Quaternion {
    pub fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self {
            inner: Inner::from_xyzw(x, y, z, w),
        }
    }

    pub fn from_angle_axis(axis: Vector3, angle: f32) -> Self {
        let d = axis.length();
        if d == 0.0 {
            Self {
                inner: Inner::from_xyzw(0.0, 0.0, 0.0, 0.0),
            }
        } else {
            let sin_angle = (angle * 0.5).sin();
            let cos_angle = (angle * 0.5).cos();
            let s = sin_angle / d;
            let x = axis.x() * s;
            let y = axis.y() * s;
            let z = axis.z() * s;
            let w = cos_angle;
            Self {
                inner: Inner::from_xyzw(x, y, z, w),
            }
        }
    }

    pub fn angle_to(self, to: Self) -> f32 {
        self.inner.angle_between(to.inner)
    }

    pub fn dot(self, with: Self) -> f32 {
        self.inner.dot(with.inner)
    }

    pub fn to_exp(self) -> Self {
        let mut v = Vector3::new(self.inner.x, self.inner.y, self.inner.z);
        let theta = v.length();
        v = v.normalized();
        if theta < CMP_EPSILON || !v.is_normalized() {
            return Quaternion::new(0.0, 0.0, 0.0, 1.0);
        }
        Quaternion::from_angle_axis(v, theta)
    }

    pub fn from_euler(self, euler: Vector3) -> Self {
        let half_a1 = euler.y() * 0.5;
        let half_a2 = euler.x() * 0.5;
        let half_a3 = euler.z() * 0.5;
        let cos_a1 = half_a1.cos();
        let sin_a1 = half_a1.sin();
        let cos_a2 = half_a2.cos();
        let sin_a2 = half_a2.sin();
        let cos_a3 = half_a3.cos();
        let sin_a3 = half_a3.sin();
        Quaternion::new(
            sin_a1 * cos_a2 * sin_a3 + cos_a1 * sin_a2 * cos_a3,
            sin_a1 * cos_a2 * cos_a3 - cos_a1 * sin_a2 * sin_a3,
            -sin_a1 * sin_a2 * cos_a3 + cos_a1 * cos_a2 * sin_a3,
            sin_a1 * sin_a2 * sin_a3 + cos_a1 * cos_a2 * cos_a3,
        )
    }

    pub fn get_angle(self) -> f32 {
        2.0 * self.inner.w.acos()
    }

    pub fn get_axis(self) -> Vector3 {
        if self.inner.w.abs() > 1.0 - CMP_EPSILON {
            return Vector3::new(self.inner.x, self.inner.y, self.inner.z);
        }
        let r = 1.0 / (1.0 - self.inner.w * self.inner.w).sqrt();
        Vector3::new(self.inner.x * r, self.inner.y * r, self.inner.z * r)
    }

    /// TODO: Figure out how godot actually treats "order", then make a match/if chain
    pub fn get_euler(self, order: Option<i32>) -> Vector3 {
        let _o = order.unwrap_or(2);
        let vt = self.inner.to_euler(glam::EulerRot::XYZ);
        Vector3::new(vt.0, vt.1, vt.2)
    }

    pub fn inverse(self) -> Self {
        Quaternion::new(-self.inner.x, -self.inner.y, -self.inner.z, self.inner.w)
    }

    pub fn is_equal_approx(self, to: Self) -> bool {
        is_equal_approx(self.inner.x, to.inner.x)
            && is_equal_approx(self.inner.y, to.inner.y)
            && is_equal_approx(self.inner.z, to.inner.z)
            && is_equal_approx(self.inner.w, to.inner.w)
    }

    pub fn is_finite(self) -> bool {
        self.inner.x.is_finite()
            && self.inner.y.is_finite()
            && self.inner.z.is_finite()
            && self.inner.w.is_finite()
    }

    pub fn is_normalized(self) -> bool {
        is_equal_approx(self.length_squared(), 1.0) /*,UNIT_EPSILON)*/
    }

    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }

    pub fn length_squared(self) -> f32 {
        self.dot(self)
    }

    pub fn log(self) -> Self {
        let v = self.get_axis() * self.get_angle();
        Quaternion::new(v.x(), v.y(), v.z(), 0.0)
    }

    pub fn normalized(self) -> Self {
        self / self.length()
    }

    pub fn slerp(self, to: Self, weight: f32) -> Self {
        let mut cosom = self.dot(to);
        let to1: Self;
        let omega: f32;
        let sinom: f32;
        let scale0: f32;
        let scale1: f32;
        if cosom < 0.0 {
            cosom = -cosom;
            to1 = -to;
        } else {
            to1 = to;
        }

        if 1.0 - cosom > CMP_EPSILON {
            omega = cosom.acos();
            sinom = omega.sin();
            scale0 = ((1.0 - weight) * omega).sin() / sinom;
            scale1 = (weight * omega).sin() / sinom;
        } else {
            scale0 = 1.0 - weight;
            scale1 = weight;
        }
        Quaternion::new(
            scale0 * self.inner.x + scale1 * to1.inner.x,
            scale0 * self.inner.y + scale1 * to1.inner.y,
            scale0 * self.inner.z + scale1 * to1.inner.z,
            scale0 * self.inner.w + scale1 * to1.inner.w,
        )
    }

    pub fn slerpni(self, to: Self, weight: f32) -> Self {
        let dot = self.dot(to);
        if dot.abs() > 0.9999 {
            return self;
        }
        let theta = dot.acos();
        let sin_t = 1.0 / theta.sin();
        let new_factor = (weight * theta).sin() * sin_t;
        let inv_factor = ((1.0 - weight) * theta).sin() * sin_t;
        Quaternion::new(
            inv_factor * self.inner.x + new_factor * to.inner.x,
            inv_factor * self.inner.y + new_factor * to.inner.y,
            inv_factor * self.inner.z + new_factor * to.inner.z,
            inv_factor * self.inner.w + new_factor * to.inner.w,
        )
    }

    // TODO: Implement godot's function in rust
    // pub fn spherical_cubic_interpolate(self, b: Self, pre_a: Self, post_b: Self, weight: f32) -> Self {}
    // TODO: Implement godot's function in rust
    /*
        pub fn spherical_cubic_interpolate_in_time(
            self,
            b: Self,
            pre_a: Self,
            post_b: Self,
            weight: f32,
            b_t: f32,
            pre_a_t: f32,
            post_b_t: f32,
        ) -> Self {
        }
    */

}

impl Add for Quaternion {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self::new(
            self.inner.x + other.inner.x,
            self.inner.y + other.inner.y,
            self.inner.z + other.inner.z,
            self.inner.w + other.inner.w,
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
            self.inner.x - other.inner.x,
            self.inner.y - other.inner.y,
            self.inner.z - other.inner.z,
            self.inner.w - other.inner.w,
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
        let x = self.inner.w * other.inner.x
            + self.inner.x * other.inner.w
            + self.inner.y * other.inner.z
            - self.inner.z * other.inner.y;
        let y = self.inner.w * other.inner.y
            + self.inner.y * other.inner.w
            + self.inner.z * other.inner.x
            - self.inner.x * other.inner.z;
        let z = self.inner.w * other.inner.z
            + self.inner.z * other.inner.w
            + self.inner.x * other.inner.y
            - self.inner.y * other.inner.x;
        let w = self.inner.w * other.inner.w
            - self.inner.x * other.inner.x
            - self.inner.y * other.inner.y
            - self.inner.z * other.inner.z;
        Self::new(x, y, z, w)
    }
}

impl MulAssign<Quaternion> for Quaternion {
    fn mul_assign(&mut self, other: Quaternion) {
        *self = *self * other
    }
}

impl Mul<f32> for Quaternion {
    type Output = Self;

    fn mul(self, other: f32) -> Self {
        Quaternion::new(
            self.inner.x * other,
            self.inner.y * other,
            self.inner.z * other,
            self.inner.w * other,
        )
    }
}

impl MulAssign<f32> for Quaternion {
    fn mul_assign(&mut self, other: f32) {
        *self = *self * other
    }
}

impl Div<f32> for Quaternion {
    type Output = Self;

    fn div(self, other: f32) -> Self {
        Self::new(
            self.inner.x / other,
            self.inner.y / other,
            self.inner.z / other,
            self.inner.w / other,
        )
    }
}

impl DivAssign<f32> for Quaternion {
    fn div_assign(&mut self, other: f32) {
        *self = *self / other
    }
}

impl Neg for Quaternion {
    type Output = Self;

    fn neg(self) -> Self {
        Self::new(-self.inner.x, -self.inner.y, -self.inner.z, -self.inner.w)
    }
}

impl GodotFfi for Quaternion {
    ffi_methods! { type sys::GDExtensionTypePtr = *mut Self; .. }
}

impl std::fmt::Display for Quaternion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}
