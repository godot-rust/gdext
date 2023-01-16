/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
use std::ops::*;

use crate::builtin::math::*;
use crate::builtin::real::Real;

impl_vector!(Vector2, crate::builtin::real::Vec2, Real, (x, y));
impl_float_vector!(Vector2, Real);
impl_vector_from!(Vector2, Vector2i, Real, (x, y));

type Inner = crate::builtin::real::Vec2;

impl Vector2 {

    /// Left unit vector. Represents the direction of left.
    pub const LEFT: Self = Self::new(-1.0, 0.0);

    /// Right unit vector. Represents the direction of right.
    pub const RIGHT: Self = Self::new(1.0, 0.0);

    /// Up unit vector. Y is down in 2D, so this vector points -Y.
    pub const UP: Self = Self::new(0.0, -1.0);

    /// Down unit vector. Y is down in 2D, so this vector points +Y.
    pub const DOWN: Self = Self::new(0.0, 1.0);
    
    pub fn abs(self) -> Self {
        Self(self.0.abs())
    }

    pub fn angle(self) -> Real {
        self.y().atan2(self.x())
    }

    pub fn angle_to(self, to: Self) -> Real {
        self.0.angle_between(to.0)
    }

    pub fn angle_to_point(self, to: Self) -> Real {
        (to - self).angle()
    }

    pub fn aspect(self) -> Real {
        self.x() / self.y()
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

	Self::new(x, y)
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

	Self::new(x, y)
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

    pub fn cross(self, with: Self) -> Real {
        self.0.perp_dot(with.0)
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

	Self::new(x, y)
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

	Self::new(x, y)
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

    pub fn dot(self, other: Self) -> Real {
        self.0.dot(other.0)
    }

    pub fn floor(self) -> Self {
        Self(self.0.floor())
    }

    pub fn from_angle(angle: Real) -> Self {
        Self(Inner::from_angle(angle))
    }

    pub fn is_equal_approx(self, to: Self) -> bool {
        is_equal_approx(self.x(), to.x()) && is_equal_approx(self.y(), to.y())
    }

    pub fn is_finite(self) -> bool {
        self.0.is_finite()
    }

    pub fn is_normalized(self) -> bool {
        self.0.is_normalized()
    }

    pub fn is_zero_approx(self) -> bool {
        is_zero_approx(self.x()) && is_zero_approx(self.y())
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
        if self.0.max_element() == self.x() {
            0
        } else {
            1
        }
    }

    pub fn min_axis_index(self) -> i32 {
        if self.0.min_element() == self.x() {
            0
        } else {
            1
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

    pub fn orthogonal(self) -> Self {
        Self::new(self.y(), -self.x())
    }

    pub fn posmod(self, pmod: Real) -> Self {
        Self::new(fposmod(self.x(), pmod), fposmod(self.y(), pmod))
    }

    pub fn posmodv(self, modv: Self) -> Self {
        Self::new(
            fposmod(self.x(), modv.x()),
            fposmod(self.y(), modv.y()),
        )
    }

    pub fn project(self, b: Self) -> Self {
        Self(self.0.project_onto(b.0))
    }

    pub fn reflect(self, normal: Self) -> Self {
        Self(self.0.reject_from(normal.0))
    }

    pub fn round(self) -> Self {
        Self(self.0.round())
    }

    pub fn sign(self) -> Self {
        -self
    }

    pub fn slerp(self, to: Self, weight: Real) -> Self {
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
        Self::new(
            snapped(self.x(), step.x()),
            snapped(self.y(), step.y()),
        )
    }

    pub fn rotated(self, angle: Real) -> Self {
        glam::Affine2::from_angle(angle).transform_vector2(self.into()).into()
    }
}

impl_vector!(Vector2i, glam::IVec2, i32, (x, y));
impl_vector_from!(Vector2i, Vector2, i32, (x, y));
