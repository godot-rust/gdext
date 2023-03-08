/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use super::real_consts::TAU;

use super::real;
use super::Vector2;

pub const CMP_EPSILON: real = 0.00001;

pub fn lerp(a: real, b: real, t: real) -> real {
    a + ((b - a) * t)
}

pub fn is_equal_approx(a: real, b: real) -> bool {
    if a == b {
        return true;
    }
    let mut tolerance = CMP_EPSILON * a.abs();
    if tolerance < CMP_EPSILON {
        tolerance = CMP_EPSILON;
    }
    (a - b).abs() < tolerance
}

/// Check if two angles are approximately equal, by comparing the distance
/// between the points on the unit circle with 0 using [`is_equal_approx`].
pub fn is_angle_equal_approx(a: real, b: real) -> bool {
    let (x1, y1) = a.sin_cos();
    let (x2, y2) = b.sin_cos();

    println!("({x1}, {y1}) ({x2}, {y2})");

    is_equal_approx(
        Vector2::distance_to(Vector2::new(x1, y1), Vector2::new(x2, y2)),
        0.0,
    )
}

pub fn is_zero_approx(s: real) -> bool {
    s.abs() < CMP_EPSILON
}

pub fn fposmod(x: real, y: real) -> real {
    let mut value = x % y;
    if ((value < 0.0) && (y > 0.0)) || ((value > 0.0) && (y < 0.0)) {
        value += y;
    }
    value += 0.0;
    value
}

pub fn snapped(mut value: real, step: real) -> real {
    if step != 0.0 {
        value = ((value / step + 0.5) * step).floor()
    }
    value
}

pub fn sign(value: real) -> real {
    if value == 0.0 {
        0.0
    } else if value < 0.0 {
        -1.0
    } else {
        1.0
    }
}

pub fn bezier_derivative(
    start: real,
    control_1: real,
    control_2: real,
    end: real,
    t: real,
) -> real {
    let omt = 1.0 - t;
    let omt2 = omt * omt;
    let t2 = t * t;
    (control_1 - start) * 3.0 * omt2
        + (control_2 - control_1) * 6.0 * omt * t
        + (end - control_2) * 3.0 * t2
}

pub fn bezier_interpolate(
    start: real,
    control_1: real,
    control_2: real,
    end: real,
    t: real,
) -> real {
    let omt = 1.0 - t;
    let omt2 = omt * omt;
    let omt3 = omt2 * omt;
    let t2 = t * t;
    let t3 = t2 * t;
    start * omt3 + control_1 * omt2 * t * 3.0 + control_2 * omt * t2 * 3.0 + end * t3
}

pub fn cubic_interpolate(from: real, to: real, pre: real, post: real, weight: real) -> real {
    0.5 * ((from * 2.0)
        + (-pre + to) * weight
        + (2.0 * pre - 5.0 * from + 4.0 * to - post) * (weight * weight)
        + (-pre + 3.0 * from - 3.0 * to + post) * (weight * weight * weight))
}

#[allow(clippy::too_many_arguments)]
pub fn cubic_interpolate_in_time(
    from: real,
    to: real,
    pre: real,
    post: real,
    weight: real,
    to_t: real,
    pre_t: real,
    post_t: real,
) -> real {
    let t = lerp(0.0, to_t, weight);
    let a1 = lerp(
        pre,
        from,
        if pre_t == 0.0 {
            0.0
        } else {
            (t - pre_t) / -pre_t
        },
    );
    let a2 = lerp(from, to, if to_t == 0.0 { 0.5 } else { t / to_t });
    let a3 = lerp(
        to,
        post,
        if post_t - to_t == 0.0 {
            1.0
        } else {
            (t - to_t) / (post_t - to_t)
        },
    );
    let b1 = lerp(
        a1,
        a2,
        if to_t - pre_t == 0.0 {
            0.0
        } else {
            (t - pre_t) / (to_t - pre_t)
        },
    );
    let b2 = lerp(a2, a3, if post_t == 0.0 { 1.0 } else { t / post_t });
    lerp(b1, b2, if to_t == 0.0 { 0.5 } else { t / to_t })
}

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
pub fn lerp_angle(from: real, to: real, weight: real) -> real {
    let difference = (to - from) % TAU;
    let distance = (2.0 * difference) % TAU - difference;
    from + distance * weight
}

/// Asserts that two values are approximately equal, using the provided `func`
/// for equality checking.
#[macro_export]
macro_rules! assert_eq_approx {
    ($a:expr, $b:expr, $func:expr $(,)?) => {
        match ($a, $b) {
            (a, b) => {
                assert!(($func)(a,b), "\n  left: {:?},\n right: {:?}", $a, $b);
            }
        }
    };
    ($a:expr, $b:expr, $func:expr, $($t:tt)+) => {
        match ($a, $b) {
            (a, b) => {
                assert!(($func)(a,b), "\n  left: {:?},\n right: {:?},\n{}", $a, $b, format_args!($($t)+));
            }
        }
    };
}

/// Asserts that two values are not approximately equal, using the provided
/// `func` for equality checking.
#[macro_export]
macro_rules! assert_ne_approx {
    ($a:expr, $b:expr, $func:expr $(, $($t:tt)*)?) => {
        #[allow(clippy::redundant_closure_call)]
        {
            assert_eq_approx!($a, $b, |a,b| !($func)(a,b) $(, $($t)*)?)
        }
    };
}

#[cfg(test)]
mod test {
    use crate::builtin::real_consts::{FRAC_PI_2, PI};

    use super::*;

    #[test]
    fn equal_approx() {
        assert_eq_approx!(1.0, 1.000001, is_equal_approx);
        assert_ne_approx!(1.0, 2.0, is_equal_approx);
        assert_eq_approx!(1.0, 1.000001, is_equal_approx, "Message {}", "formatted");
        assert_ne_approx!(1.0, 2.0, is_equal_approx, "Message {}", "formatted");
    }

    #[test]
    fn angle_equal_approx() {
        assert_eq_approx!(1.0, 1.000001, is_angle_equal_approx);
        assert_eq_approx!(0.0, TAU, is_angle_equal_approx);
        assert_eq_approx!(PI, -PI, is_angle_equal_approx);
        assert_eq_approx!(4.45783, -(TAU - 4.45783), is_angle_equal_approx);
        assert_eq_approx!(31.0 * PI, -13.0 * PI, is_angle_equal_approx);
    }

    #[test]
    #[should_panic(expected = "I am inside format")]
    fn eq_approx_fail_with_message() {
        assert_eq_approx!(1.0, 2.0, is_equal_approx, "I am inside {}", "format");
    }

    #[test]
    fn lerp_angle_test() {
        assert_eq_approx!(lerp_angle(0.0, PI, 0.5), -FRAC_PI_2, is_angle_equal_approx);
        // As mentioned in the docs for `lerp_angle`, direction can be unpredictable
        // when lerping towards PI radians, this also means it's different for single vs
        // double precision floats.

        // TODO: look into if it's possible to make a more robust impl.
        #[cfg(not(feature = "double-precision"))]
        assert_eq_approx!(
            lerp_angle(0.0, PI + 3.0 * TAU, 0.5),
            FRAC_PI_2,
            is_angle_equal_approx
        );
        #[cfg(feature = "double-precision")]
        assert_eq_approx!(
            lerp_angle(0.0, PI + 3.0 * TAU, 0.5),
            -FRAC_PI_2,
            is_angle_equal_approx
        );
        let angle = PI * 2.0 / 3.0;
        assert_eq_approx!(
            lerp_angle(-5.0 * TAU, angle + 3.0 * TAU, 0.5),
            (angle / 2.0),
            is_angle_equal_approx
        );
    }
}
