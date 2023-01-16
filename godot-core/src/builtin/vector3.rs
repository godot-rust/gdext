/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
use std::ops::*;

use godot_ffi as sys;
use sys::{ffi_methods, GodotFfi};

use crate::builtin::math::*;
use crate::builtin::real::Real;

impl_vector!(Vector3, crate::builtin::real::Vec3, Real, (x, y, z));
impl_float_vector!(Vector3, Real);
impl_vector_from!(Vector3, Vector3i, Real, (x, y, z));

impl Vector3 {
    
    pub fn abs(self) -> Self {
        Self(self.0.abs())
    }

    pub fn angle_to(self, to: Self) -> Real {
        self.0.angle_between(to.0)
    }

    pub fn bezier_derivative(
        self,
        control_1: Self,
        control_2: Self,
        end: Self,
        t: Real,
    ) -> Self {
        let x = bezier_derivative(
            self.x(),
            control_1.x(),
            control_2.x(),
            end.x(),
            t,
        );
        let y = bezier_derivative(
            self.y(),
            control_1.y(),
            control_2.y(),
            end.y(),
            t,
        );
        let z = bezier_derivative(
            self.z(),
            control_1.z(),
            control_2.z(),
            end.z(),
            t,
        );

        Self::new(x, y, z)
    }

    pub fn bezier_interpolate(
        self,
        control_1: Self,
        control_2: Self,
        end: Self,
        t: Real,
    ) -> Self {
        let x = bezier_interpolate(
            self.x(),
            control_1.x(),
            control_2.x(),
            end.x(),
            t,
        );
        let y = bezier_interpolate(
            self.y(),
            control_1.y(),
            control_2.y(),
            end.y(),
            t,
        );
        let z = bezier_interpolate(
            self.z(),
            control_1.z(),
            control_2.z(),
            end.z(),
            t,
        );

	Self::new(x, y, z)
    }

    pub fn bounce(self, normal: Self) -> Self {
        -self.reflect(normal)
    }

    pub fn ceil(self) -> Self {
        Self(self.0.ceil())
    }

    pub fn clamp(self, min: Self, max: Self) -> Self {
        Self(self.0.clamp(min.0, max.0))
    }

    pub fn cross(self, with: Self) -> Self {
        Self(self.0.cross(with.0))
    }

    pub fn cubic_interpolate(self, b: Self, pre_a: Self, post_b: Self, weight: Real) -> Self {
        let x = cubic_interpolate(
            self.x(),
            b.x(),
            pre_a.x(),
            post_b.x(),
            weight,
        );
        let y = cubic_interpolate(
            self.y(),
            b.y(),
            pre_a.y(),
            post_b.y(),
            weight,
        );
        let z = cubic_interpolate(
            self.z(),
            b.z(),
            pre_a.z(),
            post_b.z(),
            weight,
        );

	Self::new(x, y, z)
    }

    pub fn cubic_interpolate_in_time(
        self,
        b: Self,
        pre_a: Self,
        post_b: Self,
        weight: Real,
        b_t: Real,
        pre_a_t: Real,
        post_b_t: Real,
    ) -> Self {
        let x = cubic_interpolate_in_time(
            self.x(),
            b.x(),
            pre_a.x(),
            post_b.x(),
            weight,
            b_t,
            pre_a_t,
            post_b_t,
        );
        let y = cubic_interpolate_in_time(
            self.y(),
            b.y(),
            pre_a.y(),
            post_b.y(),
            weight,
            b_t,
            pre_a_t,
            post_b_t,
        );
        let z = cubic_interpolate_in_time(
            self.z(),
            b.z(),
            pre_a.z(),
            post_b.z(),
            weight,
            b_t,
            pre_a_t,
            post_b_t,
        );

	Self::new(x, y, z)
    }

    pub fn direction_to(self, to: Self) -> Self {
        (to - self).normalized()
    }

    pub fn distance_squared_to(self, to: Self) -> Real {
        self.0.distance_squared(to.0)
    }

    pub fn distance_to(self, to: Self) -> Real {
        self.0.distance(to.0)
    }

    pub fn dot(self, with: Self) -> Real {
        self.0.dot(with.0)
    }

    pub fn floor(self) -> Self {
        Self(self.0.floor())
    }

    pub fn inverse(self) -> Self {
        Self::new(1.0 / self.x(), 1.0 / self.y(), 1.0 / self.z())
    }

    pub fn is_equal_approx(self, to: Self) -> bool {
        is_equal_approx(self.x(), to.x())
            && is_equal_approx(self.y(), to.y())
            && is_equal_approx(self.z(), to.z())
    }

    pub fn is_finite(self) -> bool {
        self.0.is_finite()
    }

    pub fn is_normalized(self) -> bool {
        self.0.is_normalized()
    }

    pub fn is_zero_approx(self) -> bool {
        is_zero_approx(self.x()) && is_zero_approx(self.y()) && is_zero_approx(self.z())
    }

    pub fn length_squared(self) -> Real {
        self.0.length_squared()
    }

    pub fn lerp(self, to: Self, weight: Real) -> Self {
        Self(self.0.lerp(to.0, weight))
    }

    pub fn limit_length(self, length: Option<Real>) -> Self {
        Self(self.0.clamp_length_max(length.unwrap_or(1.0)))
    }

    pub fn max_axis_index(self) -> i32 {
        let me = self.0.max_element();
        if me == self.x() {
            0
        } else if me == self.y() {
            1
        } else {
            2
        }
    }

    pub fn min_axis_index(self) -> i32 {
        let me = self.0.min_element();
        if me == self.x() {
            0
        } else if me == self.y() {
            1
        } else {
            2
        }
    }

    pub fn move_toward(self, to: Self, delta: Real) -> Self {
        let vd = to - self;
        let len = vd.length();
        if len <= delta || len < CMP_EPSILON {
            return to;
        } else {
            return self + vd / len * delta;
        };
    }

    /// TODO: Implement a rust version of godot's
    //pub fn octahedron_decode(self, uv: Vector2) -> Self {}

    /// TODO: Implement a rust version of godot's
    //pub fn octahedron_encode(self) -> Vector2 {}

    /// TODO: Implement a rust version of godot's
    // pub fn outer(self, with: Self) -> Basis {}

    pub fn posmod(self, pmod: Real) -> Self {
        Self::new(
            fposmod(self.x(), pmod),
            fposmod(self.y(), pmod),
            fposmod(self.z(), pmod),
        )
    }

    pub fn posmodv(self, modv: Self) -> Self {
        Self::new(
            fposmod(self.x(), modv.x()),
            fposmod(self.y(), modv.y()),
            fposmod(self.z(), modv.z()),
        )
    }

    pub fn project(self, b: Self) -> Self {
        Self(self.0.project_onto(b.0))
    }

    pub fn reflect(self, normal: Self) -> Self {
        Self(self.0.reject_from(normal.0))
    }

    /// TODO: Implement a rust version of godot's
    //pub fn rotated(mut self, axis: Self, angle: Real) -> Self {}

    pub fn round(self) -> Self {
        Self(self.0.round())
    }

    pub fn sign(self) -> Self {
        -self
    }

    /// TODO: Implement a rust version of godot's
    //pub fn signed_angle_to(self, to: Self, axis: Self) -> Real {}

    /// TODO: Implement a rust version of godot's (may need to implement Quaternion before this)
    //pub fn slerp(self, to: Self, weight: Real) -> Self {}

    pub fn slide(self, normal: Self) -> Self {
        self - normal * self.dot(normal)
    }

    pub fn snapped(self, step: Self) -> Self {
        Self::new(
            snapped(self.x(), step.x()),
            snapped(self.y(), step.y()),
            snapped(self.z(), step.z()),
        )
    }

    /// Left unit vector. Represents the local direction of left, and the global direction of west.
    pub const LEFT: Self = Self::new(-1.0, 0.0, 0.0);

    /// Right unit vector. Represents the local direction of right, and the global direction of east.
    pub const RIGHT: Self = Self::new(1.0, 0.0, 0.0);

    /// Up unit vector.
    pub const UP: Self = Self::new(0.0, 1.0, 0.0);

    /// Down unit vector.
    pub const DOWN: Self = Self::new(0.0, -1.0, 0.0);

    /// Forward unit vector. Represents the local direction of forward, and the global direction of north.
    pub const FORWARD: Self = Self::new(0.0, 0.0, -1.0);

    /// Back unit vector. Represents the local direction of back, and the global direction of south.
    pub const BACK: Self = Self::new(0.0, 0.0, 1.0);
}

impl_vector!(Vector3i, glam::IVec3, i32, (x, y, z));
impl_vector_from!(Vector3i, Vector3, i32, (x, y, z));

// TODO auto-generate this, alongside all the other builtin type's enums

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(i32)]
pub enum Vector3Axis {
    X,
    Y,
    Z,
}

impl GodotFfi for Vector3Axis {
    ffi_methods! { type sys::GDExtensionTypePtr = *mut Self; .. }
}
