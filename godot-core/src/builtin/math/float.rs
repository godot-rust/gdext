/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::builtin::{real, RealConv, Vector2};

use super::ApproxEq;

mod private {
    pub trait Sealed {}

    impl Sealed for f32 {}
    impl Sealed for f64 {}
}

pub trait FloatExt: private::Sealed + Copy {
    const CMP_EPSILON: Self;

    /// Linearly interpolates from `self` to `to` by `weight`.
    ///
    /// `weight` should be in the range `0.0 ..= 1.0`, but values outside this are allowed and will perform
    /// linear extrapolation.
    fn lerp(self, to: Self, weight: Self) -> Self;

    /// Check if two angles are approximately equal, by comparing the distance
    /// between the points on the unit circle with 0 using [`is_equal_approx`].
    fn is_angle_equal_approx(self, other: Self) -> bool;

    /// Check if `self` is within [`Self::CMP_EPSILON`] of `0.0`.
    fn is_zero_approx(self) -> bool;

    fn fposmod(self, pmod: Self) -> Self;

    /// Returns the multiple of `step` that is closest to `self`.
    fn snapped(self, step: Self) -> Self;

    /// Godot's `sign` function, returns `0.0` when self is `0.0`.
    ///
    /// See also [`signum`](Self::signum), which always returns `-1.0` or `1.0` (or `NaN`).
    fn sign(self) -> Self;

    /// Returns the derivative at the given `t` on a one-dimensional Bézier curve defined by the given
    /// `control_1`, `control_2`, and `end` points.
    fn bezier_derivative(self, control_1: Self, control_2: Self, end: Self, t: Self) -> Self;

    /// Returns the point at the given `t` on a one-dimensional Bézier curve defined by the given
    /// `control_1`, `control_2`, and `end` points.
    fn bezier_interpolate(self, control_1: Self, control_2: Self, end: Self, t: Self) -> Self;

    /// Cubic interpolates between two values by the factor defined in `weight` with `pre` and `post` values.
    fn cubic_interpolate(self, to: Self, pre: Self, post: Self, weight: Self) -> Self;

    /// Cubic interpolates between two values by the factor defined in `weight` with `pre` and `post` values.
    /// It can perform smoother interpolation than [`cubic_interpolate`](FloatExt::cubic_interpolate) by the time values.
    #[allow(clippy::too_many_arguments)]
    fn cubic_interpolate_in_time(
        self,
        to: Self,
        pre: Self,
        post: Self,
        weight: Self,
        to_t: Self,
        pre_t: Self,
        post_t: Self,
    ) -> Self;

    /// Linearly interpolates between two angles (in radians) by a `weight` value
    /// between 0.0 and 1.0.
    ///
    /// Similar to [`lerp`], but interpolates correctly when the angles wrap around
    /// [`TAU`].
    ///
    /// The resulting angle is not normalized.
    ///
    /// Note: This function lerps through the shortest path between `from` and
    /// `to`. However, when these two angles are approximately `PI + k * TAU` apart
    /// for any integer `k`, it's not obvious which way they lerp due to
    /// floating-point precision errors. For example, with single-precision floats
    /// `lerp_angle(0.0, PI, weight)` lerps clockwise, while `lerp_angle(0.0, PI + 3.0 * TAU, weight)`
    /// lerps counter-clockwise.
    ///
    /// _Godot equivalent: @GlobalScope.lerp_angle()_
    fn lerp_angle(self, to: Self, weight: Self) -> Self;
}

macro_rules! impl_float_ext {
    ($Ty:ty, $consts:path, $to_real:ident) => {
        impl FloatExt for $Ty {
            const CMP_EPSILON: Self = 0.00001;

            fn lerp(self, to: Self, t: Self) -> Self {
                self + ((to - self) * t)
            }

            fn is_angle_equal_approx(self, other: Self) -> bool {
                let (x1, y1) = self.sin_cos();
                let (x2, y2) = other.sin_cos();

                let point_1 = Vector2::new(real::$to_real(x1), real::$to_real(y1));
                let point_2 = Vector2::new(real::$to_real(x2), real::$to_real(y2));

                point_1.distance_to(point_2).is_zero_approx()
            }

            fn is_zero_approx(self) -> bool {
                self.abs() < Self::CMP_EPSILON
            }

            fn fposmod(self, pmod: Self) -> Self {
                let mut value = self % pmod;
                if (value < 0.0 && pmod > 0.0) || (value > 0.0 && pmod < 0.0) {
                    value += pmod;
                }
                value
            }

            fn snapped(mut self, step: Self) -> Self {
                if step != 0.0 {
                    self = ((self / step + 0.5) * step).floor()
                }
                self
            }

            fn sign(self) -> Self {
                use std::cmp::Ordering;

                match self.partial_cmp(&0.0) {
                    Some(Ordering::Equal) => 0.0,
                    Some(Ordering::Greater) => 1.0,
                    Some(Ordering::Less) => -1.0,
                    // `self` is `NaN`
                    None => Self::NAN,
                }
            }

            fn bezier_derivative(
                self,
                control_1: Self,
                control_2: Self,
                end: Self,
                t: Self,
            ) -> Self {
                let omt = 1.0 - t;
                let omt2 = omt * omt;
                let t2 = t * t;
                (control_1 - self) * 3.0 * omt2
                    + (control_2 - control_1) * 6.0 * omt * t
                    + (end - control_2) * 3.0 * t2
            }

            fn bezier_interpolate(
                self,
                control_1: Self,
                control_2: Self,
                end: Self,
                t: Self,
            ) -> Self {
                let omt = 1.0 - t;
                let omt2 = omt * omt;
                let omt3 = omt2 * omt;
                let t2 = t * t;
                let t3 = t2 * t;
                self * omt3 + control_1 * omt2 * t * 3.0 + control_2 * omt * t2 * 3.0 + end * t3
            }

            fn cubic_interpolate(self, to: Self, pre: Self, post: Self, weight: Self) -> Self {
                0.5 * ((self * 2.0)
                    + (-pre + to) * weight
                    + (2.0 * pre - 5.0 * self + 4.0 * to - post) * (weight * weight)
                    + (-pre + 3.0 * self - 3.0 * to + post) * (weight * weight * weight))
            }

            fn cubic_interpolate_in_time(
                self,
                to: Self,
                pre: Self,
                post: Self,
                weight: Self,
                to_t: Self,
                pre_t: Self,
                post_t: Self,
            ) -> Self {
                let t = Self::lerp(0.0, to_t, weight);

                let a1 = Self::lerp(
                    pre,
                    self,
                    if pre_t == 0.0 {
                        0.0
                    } else {
                        (t - pre_t) / -pre_t
                    },
                );

                let a2 = Self::lerp(self, to, if to_t == 0.0 { 0.5 } else { t / to_t });

                let a3 = Self::lerp(
                    to,
                    post,
                    if post_t - to_t == 0.0 {
                        1.0
                    } else {
                        (t - to_t) / (post_t - to_t)
                    },
                );

                let b1 = Self::lerp(
                    a1,
                    a2,
                    if to_t - pre_t == 0.0 {
                        0.0
                    } else {
                        (t - pre_t) / (to_t - pre_t)
                    },
                );

                let b2 = Self::lerp(a2, a3, if post_t == 0.0 { 1.0 } else { t / post_t });

                Self::lerp(b1, b2, if to_t == 0.0 { 0.5 } else { t / to_t })
            }

            fn lerp_angle(self, to: Self, weight: Self) -> Self {
                use $consts;

                let difference = (to - self) % consts::TAU;
                let distance = (2.0 * difference) % consts::TAU - difference;
                self + distance * weight
            }
        }

        impl ApproxEq for $Ty {
            fn approx_eq(&self, other: &Self) -> bool {
                if self == other {
                    return true;
                }
                let mut tolerance = Self::CMP_EPSILON * self.abs();
                if tolerance < Self::CMP_EPSILON {
                    tolerance = Self::CMP_EPSILON;
                }
                (self - other).abs() < tolerance
            }
        }
    };
}

impl_float_ext!(f32, std::f32::consts, from_f32);
impl_float_ext!(f64, std::f64::consts, from_f64);

#[cfg(test)]
mod test {
    use crate::assert_eq_approx;

    use super::*;

    // Create functions that take references for use in `assert_eq/ne_approx`.
    fn is_angle_equal_approx_f32(a: &f32, b: &f32) -> bool {
        a.is_angle_equal_approx(*b)
    }

    fn is_angle_equal_approx_f64(a: &f64, b: &f64) -> bool {
        a.is_angle_equal_approx(*b)
    }

    #[test]
    fn angle_equal_approx_f32() {
        use std::f32::consts::{PI, TAU};

        assert_eq_approx!(1.0, 1.000001, fn = is_angle_equal_approx_f32);
        assert_eq_approx!(0.0, TAU, fn = is_angle_equal_approx_f32);
        assert_eq_approx!(PI, -PI, fn = is_angle_equal_approx_f32);
        assert_eq_approx!(4.45783, -(TAU - 4.45783), fn = is_angle_equal_approx_f32);
        assert_eq_approx!(31.0 * PI, -13.0 * PI, fn = is_angle_equal_approx_f32);
    }

    #[test]
    fn angle_equal_approx_f64() {
        use std::f64::consts::{PI, TAU};

        assert_eq_approx!(1.0, 1.000001, fn = is_angle_equal_approx_f64);
        assert_eq_approx!(0.0, TAU, fn = is_angle_equal_approx_f64);
        assert_eq_approx!(PI, -PI, fn = is_angle_equal_approx_f64);
        assert_eq_approx!(4.45783, -(TAU - 4.45783), fn = is_angle_equal_approx_f64);
        assert_eq_approx!(31.0 * PI, -13.0 * PI, fn = is_angle_equal_approx_f64);
    }

    #[test]
    #[should_panic(expected = "I am inside format")]
    fn eq_approx_fail_with_message() {
        assert_eq_approx!(1.0, 2.0, "I am inside {}", "format");
    }

    // As mentioned in the docs for `lerp_angle`, direction can be unpredictable
    // when lerping towards PI radians, this also means it's different for single vs
    // double precision floats.

    #[test]
    fn lerp_angle_test_f32() {
        use std::f32::consts::{FRAC_PI_2, PI, TAU};

        assert_eq_approx!(f32::lerp_angle(0.0, PI, 0.5), -FRAC_PI_2, fn = is_angle_equal_approx_f32);

        assert_eq_approx!(
            f32::lerp_angle(0.0, PI + 3.0 * TAU, 0.5),
            FRAC_PI_2,
            fn = is_angle_equal_approx_f32
        );

        let angle = PI * 2.0 / 3.0;
        assert_eq_approx!(
            f32::lerp_angle(-5.0 * TAU, angle + 3.0 * TAU, 0.5),
            (angle / 2.0),
            fn = is_angle_equal_approx_f32
        );
    }

    #[test]
    fn lerp_angle_test_f64() {
        use std::f64::consts::{FRAC_PI_2, PI, TAU};

        assert_eq_approx!(f64::lerp_angle(0.0, PI, 0.5), -FRAC_PI_2, fn = is_angle_equal_approx_f64);

        assert_eq_approx!(
            f64::lerp_angle(0.0, PI + 3.0 * TAU, 0.5),
            -FRAC_PI_2,
            fn = is_angle_equal_approx_f64
        );

        let angle = PI * 2.0 / 3.0;
        assert_eq_approx!(
            f64::lerp_angle(-5.0 * TAU, angle + 3.0 * TAU, 0.5),
            (angle / 2.0),
            fn = is_angle_equal_approx_f64
        );
    }
}
