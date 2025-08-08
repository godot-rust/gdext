/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::builtin::inner::InnerVector3;
use godot::builtin::math::{assert_eq_approx, ApproxEq};
use godot::builtin::real_consts::{FRAC_PI_4, PI};
use godot::builtin::{real, Vector3, Vector3Axis};

use crate::framework::itest;

#[itest]
fn abs() {
    let a = Vector3::new(-1.0, 2.0, -0.0);

    assert_eq!(a.abs(), a.as_inner().abs());
}

#[itest]
fn angle_to() {
    let a = Vector3::new(1.2, -3.4, 5.6);
    let b = Vector3::new(-7.8, 9.1, -11.12);

    assert_eq_approx!(a.angle_to(b), a.as_inner().angle_to(b) as real);

    // Concrete example (135°).
    let right = Vector3::new(1.0, 0.0, 0.0);
    let back_left = Vector3::new(-1.0, 0.0, 1.0);

    assert_eq_approx!(right.angle_to(back_left), 3.0 * FRAC_PI_4);
    assert_eq_approx!(back_left.angle_to(right), 3.0 * FRAC_PI_4);
}

#[itest]
fn signed_angle_to() {
    let a = Vector3::new(1.0, 1.0, 0.0);
    let b = Vector3::new(1.0, 1.0, 1.0);
    let c = Vector3::UP;

    assert_eq_approx!(
        a.signed_angle_to(b, c),
        a.as_inner().signed_angle_to(b, c) as real,
        "signed_angle_to\n",
    );

    // Concrete example (135°).
    let right = Vector3::new(1.0, 0.0, 0.0);
    let back_left = Vector3::new(-1.0, 0.0, 1.0);

    let pi_3_4 = 3.0 * FRAC_PI_4;
    assert_eq_approx!(right.signed_angle_to(back_left, Vector3::UP), -pi_3_4);
    assert_eq_approx!(right.signed_angle_to(back_left, Vector3::DOWN), pi_3_4);
    assert_eq_approx!(back_left.signed_angle_to(right, Vector3::UP), pi_3_4);
    assert_eq_approx!(back_left.signed_angle_to(right, Vector3::DOWN), -pi_3_4);
}

#[itest]
fn bezier_derivative() {
    let a = Vector3::new(1.2, -3.4, 5.6);
    let b = Vector3::new(-7.8, 9.1, -11.12);
    let c = Vector3::new(13.0, -14.0, 15.0);
    let d = Vector3::new(-16.0, 17.0, -18.0);

    let e = 10.0;

    assert_eq!(
        a.bezier_derivative(b, c, d, e as real),
        a.as_inner().bezier_derivative(b, c, d, e)
    );
}

#[itest]
fn bezier_interpolate() {
    let a = Vector3::new(1.2, -3.4, 5.6);
    let b = Vector3::new(-7.8, 9.10, -11.12);
    let c = Vector3::new(13.0, -14.0, 15.0);
    let d = Vector3::new(-16.0, 17.0, -18.0);

    let e = 10.0;

    assert_eq!(
        a.bezier_interpolate(b, c, d, e as real),
        a.as_inner().bezier_interpolate(b, c, d, e)
    );
}

#[itest]
fn bounce() {
    let a = Vector3::new(1.2, -3.4, 5.6);
    let b = Vector3::new(-7.8, 9.10, -11.12).normalized();

    assert_eq_approx!(a.bounce(b), a.as_inner().bounce(b));
}

#[itest]
fn ceil() {
    let a = Vector3::new(1.2, -3.4, 5.6);

    assert_eq!(a.ceil(), a.as_inner().ceil());
}

#[itest]
fn clamp() {
    let a = Vector3::new(12.3, 45.6, 78.9);

    let min = Vector3::new(15.0, 15.0, 15.0);
    let max = Vector3::new(30.0, 30.0, 30.0);

    assert_eq!(a.clamp(min, max), a.as_inner().clamp(min, max));
}

#[itest]
fn cross() {
    let a = Vector3::new(1.2, -3.4, 5.6);
    let b = Vector3::new(-7.8, 9.10, -11.12);

    assert_eq_approx!(a.cross(b), a.as_inner().cross(b));
}

#[itest]
fn cubic_interpolate() {
    let a = Vector3::new(1.0, 2.0, 3.0);
    let b = Vector3::new(4.0, 5.0, 6.0);
    let c = Vector3::new(0.0, 1.0, 2.0);
    let d = Vector3::new(5.0, 6.0, 7.0);

    let e = 0.5;

    assert_eq!(
        a.cubic_interpolate(b, c, d, e as real),
        a.as_inner().cubic_interpolate(b, c, d, e)
    );
}

#[itest]
fn cubic_interpolate_in_time() {
    let a = Vector3::new(1.0, 2.0, 3.0);
    let b = Vector3::new(4.0, 5.0, 6.0);
    let c = Vector3::new(0.0, 1.0, 2.0);
    let d = Vector3::new(5.0, 6.0, 7.0);

    let e = 0.5;
    let f = 0.3;
    let g = 0.2;
    let h = 0.4;

    assert_eq_approx!(
        a.cubic_interpolate_in_time(b, c, d, e as real, f as real, g as real, h as real),
        a.as_inner().cubic_interpolate_in_time(b, c, d, e, f, g, h)
    );
}

#[itest]
fn direction_to() {
    let a = Vector3::new(1.2, -3.4, 5.6);
    let b = Vector3::new(-7.8, 9.10, -11.12);

    assert_eq!(a.direction_to(b), a.as_inner().direction_to(b));
}

#[itest]
fn distance_squared_to() {
    let a = Vector3::new(1.2, -3.4, 5.6);
    let b = Vector3::new(-7.8, 9.10, -11.12);

    assert_eq!(
        a.distance_squared_to(b),
        a.as_inner().distance_squared_to(b) as real
    );
}

#[itest]
fn distance_to() {
    let a = Vector3::new(1.2, -3.4, 5.6);
    let b = Vector3::new(-7.8, 9.10, -11.12);

    assert_eq!(a.distance_to(b), a.as_inner().distance_to(b) as real);
}

#[itest]
fn dot() {
    let a = Vector3::new(1.2, -3.4, 5.6);
    let b = Vector3::new(-7.8, 9.10, -11.12);

    assert_eq_approx!(a.dot(b), a.as_inner().dot(b) as real);
}

#[itest]
fn floor() {
    let a = Vector3::new(1.2, -3.4, 5.6);

    assert_eq!(a.floor(), a.as_inner().floor());
}

#[itest]
fn inverse() {
    let a = Vector3::new(1.2, -3.4, 5.6);

    assert_eq!(a.recip(), a.as_inner().inverse());
}

#[itest]
fn is_equal_approx() {
    let a = Vector3::new(1.2, -3.4, 5.6);
    let b = a * 1.000001;

    let c = Vector3::new(-7.8, 9.10, -11.12);

    assert_eq!(a.approx_eq(&b), a.as_inner().is_equal_approx(b));
    assert_eq!(a.approx_eq(&c), a.as_inner().is_equal_approx(c));
}

#[itest]
fn is_finite() {
    let a = Vector3::new(1.2, -3.4, 5.6);
    let b = Vector3::new(real::INFINITY, real::INFINITY, real::INFINITY);

    assert_eq!(a.is_finite(), a.as_inner().is_finite());
    assert_eq!(b.is_finite(), b.as_inner().is_finite());
}

#[itest]
fn is_normalized() {
    let a = Vector3::new(1.2, -3.4, 5.6);
    let b = Vector3::new(1.0, 0.0, 1.0);

    assert_eq!(a.is_normalized(), a.as_inner().is_normalized());
    assert_eq!(b.is_normalized(), b.as_inner().is_normalized());
}

#[itest]
fn is_zero_approx() {
    let a = Vector3::new(1.2, -3.4, 5.6);
    let b = Vector3::new(0.0, 0.0, 0.0);

    assert_eq!(a.is_zero_approx(), a.as_inner().is_zero_approx());
    assert_eq!(b.is_zero_approx(), b.as_inner().is_zero_approx());
}

#[itest]
fn length() {
    let a = Vector3::new(1.2, -3.4, 5.6);

    assert_eq!(a.length(), a.as_inner().length() as real);
}

#[itest]
fn length_squared() {
    let a = Vector3::new(1.2, -3.4, 5.6);

    assert_eq!(a.length_squared(), a.as_inner().length_squared() as real);
}

#[itest]
fn lerp() {
    let a = Vector3::new(1.2, -3.4, 5.6);
    let b = Vector3::new(-7.8, 9.10, -11.12);

    let c = 0.5;

    assert_eq!(a.lerp(b, c as real), a.as_inner().lerp(b, c));
}

#[itest]
fn limit_length() {
    let a = Vector3::new(1.2, -3.4, 5.6);
    let b = 5.0;

    assert_eq!(
        a.limit_length(Some(b as real)),
        a.as_inner().limit_length(b)
    );
}

#[itest]
fn max_axis() {
    let a = Vector3::new(10.0, 5.0, 0.0);
    let b = Vector3::new(10.0, 10.0, 10.0);

    assert_eq!(
        a.max_axis(),
        match a.as_inner().max_axis_index() {
            0 => Some(Vector3Axis::X),
            1 => Some(Vector3Axis::Y),
            2 => Some(Vector3Axis::Z),
            _ => None,
        }
    );
    assert_eq!(
        b.max_axis().unwrap_or(Vector3Axis::X),
        match b.as_inner().max_axis_index() {
            0 => Vector3Axis::X,
            1 => Vector3Axis::Y,
            2 => Vector3Axis::Z,
            _ => unreachable!(),
        }
    );
}

#[itest]
fn min_axis() {
    let a = Vector3::new(10.0, 5.0, 0.0);
    let b = Vector3::new(10.0, 10.0, 10.0);

    assert_eq!(
        a.min_axis(),
        match a.as_inner().min_axis_index() {
            0 => Some(Vector3Axis::X),
            1 => Some(Vector3Axis::Y),
            2 => Some(Vector3Axis::Z),
            _ => None,
        }
    );
    assert_eq!(
        b.min_axis().unwrap_or(Vector3Axis::Z),
        match b.as_inner().min_axis_index() {
            0 => Vector3Axis::X,
            1 => Vector3Axis::Y,
            2 => Vector3Axis::Z,
            _ => unreachable!(),
        }
    );
}

#[itest]
fn move_toward() {
    let a = Vector3::new(1.2, -3.4, 5.6);
    let b = Vector3::new(-7.8, 9.10, -11.12);

    let c = 5.0;

    assert_eq!(a.move_toward(b, c as real), a.as_inner().move_toward(b, c));
}

#[itest]
fn try_normalized() {
    let a = Vector3::new(1.2, -3.4, 5.6);
    let b = Vector3::ZERO;

    assert_eq_approx!(a.try_normalized().unwrap(), a.as_inner().normalized());
    assert_eq!(b.try_normalized(), None);
}

#[itest]
fn octahedron_encode() {
    let a = Vector3::new(1.2, -3.4, 5.6).normalized();

    assert_eq!(a.octahedron_encode(), a.as_inner().octahedron_encode());
}

#[itest]
fn orthogonal_decode() {
    let a = Vector3::new(1.2, -3.4, 5.6)
        .normalized()
        .octahedron_encode();

    assert_eq!(
        Vector3::octahedron_decode(a),
        InnerVector3::octahedron_decode(a)
    );
}

#[itest]
fn outer() {
    let a = Vector3::new(1.2, -3.4, 5.6);
    let b = Vector3::new(-7.8, 9.10, -11.12);

    assert_eq!(a.outer(b), a.as_inner().outer(b));
}

#[itest]
fn posmod() {
    let a = Vector3::new(1.2, -3.4, 5.6);
    let b = 5.6;

    assert_eq!(a.posmod(b as real), a.as_inner().posmod(b));
}

#[itest]
fn posmodv() {
    let a = Vector3::new(1.2, -3.4, 5.6);
    let b = Vector3::new(-7.8, 9.10, -11.12);

    assert_eq!(a.posmodv(b), a.as_inner().posmodv(b));
}

#[itest]
fn project() {
    let a = Vector3::new(1.2, -3.4, 5.6);
    let b = Vector3::new(-7.8, 9.10, -11.12);

    assert_eq_approx!(a.project(b), a.as_inner().project(b));
}

#[itest]
fn reflect() {
    let a = Vector3::new(1.2, -3.4, 5.6);
    let b = Vector3::new(-7.8, 9.10, -11.12).normalized();

    assert_eq_approx!(a.reflect(b), a.as_inner().reflect(b));
}

#[itest]
fn rotated() {
    let a = Vector3::new(1.2, -3.4, 5.6);

    let b = Vector3::UP;
    let c = 1.0;

    assert_eq!(a.rotated(b, c as real), a.as_inner().rotated(b, c));
}

#[itest]
fn round() {
    let a = Vector3::new(1.2, -3.6, 7.8);

    assert_eq!(a.round(), a.as_inner().round());
}

#[itest]
fn sign() {
    let a = Vector3::new(-1.0, 2.0, -3.0);
    let b = Vector3::new(-0.0, 0.0, -0.0);

    assert_eq!(a.sign(), a.as_inner().sign());
    assert_eq!(b.sign(), b.as_inner().sign());
}

#[itest]
fn slerp() {
    let a = Vector3::new(1.2, -3.4, 5.6);
    let b = Vector3::new(-7.8, 9.10, -11.12);

    let c = 0.5;

    assert_eq_approx!(a.slerp(b, c as real), a.as_inner().slerp(b, c));
}

#[itest]
fn slide() {
    let a = Vector3::new(1.2, -3.4, 5.6);
    let b = Vector3::new(-7.8, 9.10, -11.12).normalized();

    assert_eq_approx!(a.slide(b), a.as_inner().slide(b));
}

#[itest]
fn snapped() {
    let a = Vector3::new(1.2, -3.4, 5.6);
    let b = Vector3::new(-7.8, 9.10, -11.12);

    assert_eq!(a.snapped(b), a.as_inner().snapped(b));
}

#[itest]
fn equiv() {
    for c in 0..10 {
        let angle = 0.2 * c as real * PI;
        let z = 0.2 * c as real - 1.0;

        let outer = Vector3::new(angle.cos(), angle.sin(), z);
        let inner = InnerVector3::from_outer(&outer);

        let x_axis = Vector3::new(1.0, 0.0, 0.0);
        let y_axis = Vector3::new(0.0, 1.0, 0.0);
        let z_axis = Vector3::new(0.0, 0.0, 1.0);

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

        assert_eq_approx!(
            outer.reflect(z_axis),
            inner.reflect(z_axis),
            "reflect (z-axis)\n",
        );
    }
}
