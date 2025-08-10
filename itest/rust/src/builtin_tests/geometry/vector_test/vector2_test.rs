/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::builtin::inner::InnerVector2;
use godot::builtin::math::{assert_eq_approx, ApproxEq};
use godot::builtin::real_consts::{FRAC_PI_2, PI};
use godot::builtin::{real, Vector2, Vector2Axis};

use crate::framework::itest;

#[itest]
fn abs() {
    let a = Vector2::new(-1.0, 2.0);

    assert_eq!(a.abs(), a.as_inner().abs());
}

#[itest]
fn angle() {
    let a = Vector2::new(1.2, -3.4);
    let b = Vector2::new(-5.6, 7.8);

    assert_eq!(a.angle(), a.as_inner().angle() as real);
    assert_eq!(b.angle(), b.as_inner().angle() as real);

    // Check direction (note: DOWN=(0, 1)).
    assert_eq_approx!(Vector2::RIGHT.angle(), 0.0);
    assert_eq_approx!(Vector2::DOWN.angle(), FRAC_PI_2);
    assert_eq_approx!(Vector2::LEFT.angle(), PI);
    assert_eq_approx!(Vector2::UP.angle(), -FRAC_PI_2);
}

#[itest]
fn angle_to() {
    let a = Vector2::new(1.2, -3.4);
    let b = Vector2::new(-5.6, 7.8);

    assert_eq_approx!(a.angle_to(b), a.as_inner().angle_to(b) as real);
    assert_eq_approx!(b.angle_to(a), b.as_inner().angle_to(a) as real);

    // Check direction (note: DOWN=(0, 1)).
    assert_eq_approx!(Vector2::RIGHT.angle_to(Vector2::DOWN), FRAC_PI_2);
    assert_eq_approx!(Vector2::DOWN.angle_to(Vector2::RIGHT), -FRAC_PI_2);
}

#[itest]
fn angle_to_point() {
    let a = Vector2::new(1.2, -3.4);
    let b = Vector2::new(-5.6, 7.8);

    assert_eq!(a.angle_to_point(b), a.as_inner().angle_to_point(b) as real);
    assert_eq!(b.angle_to_point(a), b.as_inner().angle_to_point(a) as real);

    // Check absolute value.
    assert_eq_approx!(
        Vector2::new(1.0, 1.0).angle_to_point(Vector2::new(1.0, 2.0)),
        FRAC_PI_2
    ); // up
    assert_eq_approx!(
        Vector2::new(1.0, 1.0).angle_to_point(Vector2::new(1.0, -1.0)),
        -FRAC_PI_2
    ); // down
}

#[itest]
fn aspect() {
    let a = Vector2::new(4.0, 2.0);

    assert_eq!(a.aspect(), a.as_inner().aspect() as real);
}

#[itest]
fn bezier_derivative() {
    let a = Vector2::new(1.2, -3.4);
    let b = Vector2::new(-5.6, 7.8);
    let c = Vector2::new(9.0, -10.0);
    let d = Vector2::new(-11.0, 12.0);

    let e = 10.0;

    assert_eq_approx!(
        a.bezier_derivative(b, c, d, e as real),
        a.as_inner().bezier_derivative(b, c, d, e)
    );
}

#[itest]
fn bezier_interpolate() {
    let a = Vector2::new(1.2, -3.4);
    let b = Vector2::new(-5.6, 7.8);
    let c = Vector2::new(9.0, -10.0);
    let d = Vector2::new(-11.0, 12.0);

    let e = 10.0;

    assert_eq!(
        a.bezier_interpolate(b, c, d, e as real),
        a.as_inner().bezier_interpolate(b, c, d, e)
    );
}

#[itest]
fn bounce() {
    let a = Vector2::new(1.2, -3.4);
    let b = Vector2::new(-5.6, 7.8).normalized();

    assert_eq!(a.bounce(b), a.as_inner().bounce(b));
}

#[itest]
fn ceil() {
    let a = Vector2::new(1.2, -3.4);

    assert_eq!(a.ceil(), a.as_inner().ceil());
}

#[itest]
fn clamp() {
    let a = Vector2::new(12.3, 45.6);

    let min = Vector2::new(15.0, 15.0);
    let max = Vector2::new(30.0, 30.0);

    assert_eq!(a.clamp(min, max), a.as_inner().clamp(min, max));
}

#[itest]
fn cross() {
    let a = Vector2::new(1.2, -3.4);
    let b = Vector2::new(-5.6, 7.8);

    assert_eq!(a.cross(b), a.as_inner().cross(b) as real);
}

#[itest]
fn cubic_interpolate() {
    let a = Vector2::new(1.0, 2.0);
    let b = Vector2::new(3.0, 4.0);
    let c = Vector2::new(0.0, 1.0);
    let d = Vector2::new(5.0, 6.0);

    let e = 0.5;

    assert_eq!(
        a.cubic_interpolate(b, c, d, e as real),
        a.as_inner().cubic_interpolate(b, c, d, e)
    );
}

#[itest]
fn cubic_interpolate_in_time() {
    let a = Vector2::new(1.0, 2.0);
    let b = Vector2::new(3.0, 4.0);
    let c = Vector2::new(0.0, 1.0);
    let d = Vector2::new(5.0, 6.0);

    let e = 0.5;
    let f = 0.3;
    let g = 0.2;
    let h = 0.4;

    assert_eq!(
        a.cubic_interpolate_in_time(b, c, d, e as real, f as real, g as real, h as real),
        a.as_inner().cubic_interpolate_in_time(b, c, d, e, f, g, h)
    );
}

#[itest]
fn direction_to() {
    let a = Vector2::new(1.2, -3.4);
    let b = Vector2::new(-5.6, 7.8);

    assert_eq!(a.direction_to(b), a.as_inner().direction_to(b));
}

#[itest]
fn distance_squared_to() {
    let a = Vector2::new(1.2, -3.4);
    let b = Vector2::new(-5.6, 7.8);

    assert_eq!(
        a.distance_squared_to(b),
        a.as_inner().distance_squared_to(b) as real
    );
}

#[itest]
fn distance_to() {
    let a = Vector2::new(1.2, -3.4);
    let b = Vector2::new(-5.6, 7.8);

    assert_eq!(a.distance_to(b), a.as_inner().distance_to(b) as real);
}

#[itest]
fn dot() {
    let a = Vector2::new(1.2, -3.4);
    let b = Vector2::new(-5.6, 7.8);

    assert_eq!(a.dot(b), a.as_inner().dot(b) as real);
}

#[itest]
fn floor() {
    let a = Vector2::new(1.2, -3.4);

    assert_eq!(a.floor(), a.as_inner().floor());
}

#[itest]
fn from_angle() {
    let a = 1.2;

    assert_eq!(Vector2::from_angle(a as real), InnerVector2::from_angle(a));
}

#[itest]
fn is_equal_approx() {
    let a = Vector2::new(1.2, -3.4);
    let b = a * 1.000001;

    let c = Vector2::new(-5.6, 7.8);

    assert_eq!(a.approx_eq(&b), a.as_inner().is_equal_approx(b));
    assert_eq!(a.approx_eq(&c), a.as_inner().is_equal_approx(c));
}

#[itest]
fn is_finite() {
    let a = Vector2::new(1.2, -3.4);
    let b = Vector2::new(real::INFINITY, real::INFINITY);

    assert_eq!(a.is_finite(), a.as_inner().is_finite());
    assert_eq!(b.is_finite(), b.as_inner().is_finite());
}

#[itest]
fn is_normalized() {
    let a = Vector2::new(1.2, -3.4);
    let b = Vector2::new(1.0, 0.0);

    assert_eq!(a.is_normalized(), a.as_inner().is_normalized());
    assert_eq!(b.is_normalized(), b.as_inner().is_normalized());
}

#[itest]
fn is_zero_approx() {
    let a = Vector2::new(1.2, -3.4);
    let b = Vector2::new(0.0, 0.0);

    assert_eq!(a.is_zero_approx(), a.as_inner().is_zero_approx());
    assert_eq!(b.is_zero_approx(), b.as_inner().is_zero_approx());
}

#[itest]
fn length() {
    let a = Vector2::new(1.2, -3.4);

    assert_eq_approx!(a.length(), a.as_inner().length() as real);
}

#[itest]
fn length_squared() {
    let a = Vector2::new(1.2, -3.4);

    assert_eq_approx!(a.length_squared(), a.as_inner().length_squared() as real);
}

#[itest]
fn lerp() {
    let a = Vector2::new(1.2, -3.4);
    let b = Vector2::new(-5.6, 7.8);

    let c = 0.5;

    assert_eq!(a.lerp(b, c as real), a.as_inner().lerp(b, c));
}

#[itest]
fn limit_length() {
    let a = Vector2::new(1.2, -3.4);
    let b = 5.0;

    assert_eq!(
        a.limit_length(Some(b as real)),
        a.as_inner().limit_length(b)
    );
}

#[itest]
fn max_axis() {
    let a = Vector2::new(10.0, 5.0);
    let b = Vector2::new(10.0, 10.0);

    assert_eq!(
        a.max_axis(),
        match a.as_inner().max_axis_index() {
            0 => Some(Vector2Axis::X),
            1 => Some(Vector2Axis::Y),
            _ => None,
        }
    );
    assert_eq!(
        b.max_axis().unwrap_or(Vector2Axis::X),
        match b.as_inner().max_axis_index() {
            0 => Vector2Axis::X,
            1 => Vector2Axis::Y,
            _ => unreachable!(),
        }
    );
}

#[itest]
fn min_axis() {
    let a = Vector2::new(10.0, 5.0);
    let b = Vector2::new(10.0, 10.0);

    assert_eq!(
        a.min_axis(),
        match a.as_inner().min_axis_index() {
            0 => Some(Vector2Axis::X),
            1 => Some(Vector2Axis::Y),
            _ => None,
        }
    );
    assert_eq!(
        b.min_axis().unwrap_or(Vector2Axis::Y),
        match b.as_inner().min_axis_index() {
            0 => Vector2Axis::X,
            1 => Vector2Axis::Y,
            _ => unreachable!(),
        }
    );
}

#[itest]
fn move_toward() {
    let a = Vector2::new(1.2, -3.4);
    let b = Vector2::new(-5.6, 7.8);

    let c = 5.0;

    assert_eq!(a.move_toward(b, c as real), a.as_inner().move_toward(b, c));
}

#[itest]
fn try_normalized() {
    let a = Vector2::new(1.2, -3.4);
    let b = Vector2::ZERO;

    assert_eq_approx!(a.try_normalized().unwrap(), a.as_inner().normalized());
    assert_eq!(b.try_normalized(), None);
}

#[itest]
fn orthogonal() {
    let a = Vector2::new(1.2, -3.4);

    assert_eq!(a.orthogonal(), a.as_inner().orthogonal());
}

#[itest]
fn posmod() {
    let a = Vector2::new(1.2, -3.4);
    let b = 5.6;

    assert_eq!(a.posmod(b as real), a.as_inner().posmod(b));
}

#[itest]
fn posmodv() {
    let a = Vector2::new(1.2, -3.4);
    let b = Vector2::new(-5.6, 7.8);

    assert_eq!(a.posmodv(b), a.as_inner().posmodv(b));
}

#[itest]
fn project() {
    let a = Vector2::new(1.2, -3.4);
    let b = Vector2::new(-5.6, 7.8);

    assert_eq_approx!(a.project(b), a.as_inner().project(b));
}

#[itest]
fn reflect() {
    let a = Vector2::new(1.2, -3.4);
    let b = Vector2::new(-5.6, 7.8).normalized();

    assert_eq!(a.reflect(b), a.as_inner().reflect(b));
}

#[itest]
fn rotated() {
    let a = Vector2::new(1.2, -3.4);
    let b = 1.0;

    assert_eq_approx!(a.rotated(b as real), a.as_inner().rotated(b));
}

#[itest]
fn round() {
    let a = Vector2::new(1.2, -3.6);

    assert_eq!(a.round(), a.as_inner().round());
}

#[itest]
fn sign() {
    let a = Vector2::new(-1.0, 2.0);
    let b = Vector2::new(-0.0, 0.0);

    assert_eq!(a.sign(), a.as_inner().sign());
    assert_eq!(b.sign(), b.as_inner().sign());
}

#[itest]
fn slerp() {
    let a = Vector2::new(1.2, -3.4);
    let b = Vector2::new(-5.6, 7.8);

    let c = 0.5;

    assert_eq_approx!(a.slerp(b, c as real), a.as_inner().slerp(b, c));
}

#[itest]
fn slide() {
    let a = Vector2::new(1.2, -3.4);
    let b = Vector2::new(-5.6, 7.8).normalized();

    assert_eq!(a.slide(b), a.as_inner().slide(b));
}

#[itest]
fn snapped() {
    let a = Vector2::new(5.0, -5.0);
    let b = Vector2::new(5.6, 7.8);

    assert_eq!(a.snapped(b), a.as_inner().snapped(b));
}

#[itest]
fn equiv() {
    for c in 0..10 {
        let angle = 0.2 * c as real * PI;

        let outer = Vector2::new(angle.cos(), angle.sin());
        let inner = InnerVector2::from_outer(&outer);

        let x_axis = Vector2::new(1.0, 0.0);
        let y_axis = Vector2::new(0.0, 1.0);

        assert_eq_approx!(
            outer.reflect(x_axis),
            inner.reflect(x_axis),
            "reflect (x-axis)\n",
        );

        assert_eq_approx!(
            outer.reflect(y_axis),
            inner.reflect(y_axis),
            "reflect (y-axis)\n",
        );
    }
}
