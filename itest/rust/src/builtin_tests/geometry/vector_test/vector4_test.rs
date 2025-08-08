/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::builtin::math::{assert_eq_approx, ApproxEq};
use godot::builtin::{real, Vector4, Vector4Axis};

use crate::framework::itest;

#[itest]
fn abs() {
    let a = Vector4::new(-1.0, 2.0, -0.0, 0.0);

    assert_eq!(a.abs(), a.as_inner().abs());
}

#[itest]
fn ceil() {
    let a = Vector4::new(1.2, -3.4, 5.6, -7.8);

    assert_eq!(a.ceil(), a.as_inner().ceil());
}

#[itest]
fn clamp() {
    let a = Vector4::new(12.3, 45.6, 78.9, 101.1);

    let min = Vector4::new(15.0, 15.0, 15.0, 15.0);
    let max = Vector4::new(30.0, 30.0, 30.0, 30.0);

    assert_eq!(a.clamp(min, max), a.as_inner().clamp(min, max));
}

#[itest]
fn cubic_interpolate() {
    let a = Vector4::new(1.0, 2.0, 3.0, 4.0);
    let b = Vector4::new(5.0, 6.0, 7.0, 8.0);
    let c = Vector4::new(0.0, 1.0, 2.0, 3.0);
    let d = Vector4::new(9.0, 10.0, 11.0, 12.0);

    let e = 0.5;

    assert_eq!(
        a.cubic_interpolate(b, c, d, e as real),
        a.as_inner().cubic_interpolate(b, c, d, e)
    );
}

#[itest]
fn cubic_interpolate_in_time() {
    let a = Vector4::new(1.0, 2.0, 3.0, 4.0);
    let b = Vector4::new(5.0, 6.0, 7.0, 8.0);
    let c = Vector4::new(0.0, 1.0, 2.0, 3.0);
    let d = Vector4::new(9.0, 10.0, 11.0, 12.0);

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
    let a = Vector4::new(1.2, -3.4, 5.6, -7.8);
    let b = Vector4::new(-9.10, 11.12, -13.14, 15.16);

    assert_eq!(a.direction_to(b), a.as_inner().direction_to(b));
}

#[itest]
fn distance_squared_to() {
    let a = Vector4::new(1.2, -3.4, 5.6, -7.8);
    let b = Vector4::new(-9.10, 11.12, -13.14, 15.16);

    assert_eq!(
        a.distance_squared_to(b),
        a.as_inner().distance_squared_to(b) as real
    );
}

#[itest]
fn distance_to() {
    let a = Vector4::new(1.2, -3.4, 5.6, -7.8);
    let b = Vector4::new(-9.10, 11.12, -13.14, 15.16);

    assert_eq!(a.distance_to(b), a.as_inner().distance_to(b) as real);
}

#[itest]
fn dot() {
    let a = Vector4::new(1.2, -3.4, 5.6, -7.8);
    let b = Vector4::new(-9.10, 11.12, -13.14, 15.16);

    assert_eq_approx!(a.dot(b), a.as_inner().dot(b) as real);
}

#[itest]
fn floor() {
    let a = Vector4::new(1.2, -3.4, 5.6, -7.8);

    assert_eq!(a.floor(), a.as_inner().floor());
}

#[itest]
fn inverse() {
    let a = Vector4::new(1.2, -3.4, 5.6, -7.8);

    assert_eq!(a.recip(), a.as_inner().inverse());
}

#[itest]
fn is_equal_approx() {
    let a = Vector4::new(1.2, -3.4, 5.6, -7.8);
    let b = a * 1.000001;

    let c = Vector4::new(-9.10, 11.12, -13.14, 15.16);

    assert_eq!(a.approx_eq(&b), a.as_inner().is_equal_approx(b));
    assert_eq!(a.approx_eq(&c), a.as_inner().is_equal_approx(c));
}

#[itest]
fn is_finite() {
    let a = Vector4::new(1.2, -3.4, 5.6, -7.8);
    let b = Vector4::new(
        real::INFINITY,
        real::INFINITY,
        real::INFINITY,
        real::INFINITY,
    );

    assert_eq!(a.is_finite(), a.as_inner().is_finite());
    assert_eq!(b.is_finite(), b.as_inner().is_finite());
}

#[itest]
fn is_normalized() {
    let a = Vector4::new(1.2, -3.4, 5.6, -7.8);
    let b = Vector4::new(1.0, 0.0, 1.0, 0.0);

    assert_eq!(a.is_normalized(), a.as_inner().is_normalized());
    assert_eq!(b.is_normalized(), b.as_inner().is_normalized());
}

#[itest]
fn is_zero_approx() {
    let a = Vector4::new(1.2, -3.4, 5.6, -7.8);
    let b = Vector4::new(0.0, 0.0, 0.0, 0.0);

    assert_eq!(a.is_zero_approx(), a.as_inner().is_zero_approx());
    assert_eq!(b.is_zero_approx(), b.as_inner().is_zero_approx());
}

#[itest]
fn length() {
    let a = Vector4::new(1.2, -3.4, 5.6, -7.8);

    assert_eq!(a.length(), a.as_inner().length() as real);
}

#[itest]
fn length_squared() {
    let a = Vector4::new(1.2, -3.4, 5.6, -7.8);

    assert_eq_approx!(a.length_squared(), a.as_inner().length_squared() as real);
}

#[itest]
fn lerp() {
    let a = Vector4::new(1.2, -3.4, 5.6, -7.8);
    let b = Vector4::new(-9.10, 11.12, -13.14, 15.16);

    let c = 0.5;

    assert_eq!(a.lerp(b, c as real), a.as_inner().lerp(b, c));
}

#[itest]
fn max_axis() {
    let a = Vector4::new(10.0, 5.0, 0.0, -5.0);
    let b = Vector4::new(10.0, 10.0, 10.0, 10.0);

    assert_eq!(
        a.max_axis(),
        match a.as_inner().max_axis_index() {
            0 => Some(Vector4Axis::X),
            1 => Some(Vector4Axis::Y),
            2 => Some(Vector4Axis::Z),
            3 => Some(Vector4Axis::W),
            _ => None,
        }
    );
    assert_eq!(
        b.max_axis().unwrap_or(Vector4Axis::X),
        match b.as_inner().max_axis_index() {
            0 => Vector4Axis::X,
            1 => Vector4Axis::Y,
            2 => Vector4Axis::Z,
            3 => Vector4Axis::W,
            _ => unreachable!(),
        }
    );
}

#[itest]
fn min_axis() {
    let a = Vector4::new(10.0, 5.0, 0.0, -5.0);
    let b = Vector4::new(10.0, 10.0, 10.0, 10.0);

    assert_eq!(
        a.min_axis(),
        match a.as_inner().min_axis_index() {
            0 => Some(Vector4Axis::X),
            1 => Some(Vector4Axis::Y),
            2 => Some(Vector4Axis::Z),
            3 => Some(Vector4Axis::W),
            _ => None,
        }
    );
    assert_eq!(
        b.min_axis().unwrap_or(Vector4Axis::W),
        match b.as_inner().min_axis_index() {
            0 => Vector4Axis::X,
            1 => Vector4Axis::Y,
            2 => Vector4Axis::Z,
            3 => Vector4Axis::W,
            _ => unreachable!(),
        }
    );
}

#[itest]
fn try_normalized() {
    let a = Vector4::new(1.2, -3.4, 5.6, -7.8);
    let b = Vector4::ZERO;

    assert_eq_approx!(a.try_normalized().unwrap(), a.as_inner().normalized());
    assert_eq!(b.try_normalized(), None);
}

#[itest]
fn posmod() {
    let a = Vector4::new(1.2, -3.4, 5.6, -7.8);
    let b = 5.6;

    assert_eq!(a.posmod(b as real), a.as_inner().posmod(b));
}

#[itest]
fn posmodv() {
    let a = Vector4::new(1.2, -3.4, 5.6, -7.8);
    let b = Vector4::new(-9.10, 11.12, -13.14, 15.16);

    assert_eq!(a.posmodv(b), a.as_inner().posmodv(b));
}

#[itest]
fn round() {
    let a = Vector4::new(1.2, -3.6, 7.8, -11.12);

    assert_eq!(a.round(), a.as_inner().round());
}

#[itest]
fn sign() {
    let a = Vector4::new(-1.0, 2.0, -3.0, 4.0);
    let b = Vector4::new(-0.0, 0.0, -0.0, 0.0);

    assert_eq!(a.sign(), a.as_inner().sign());
    assert_eq!(b.sign(), b.as_inner().sign());
}

#[itest]
fn snapped() {
    let a = Vector4::new(1.2, -3.4, 5.6, -7.8);
    let b = Vector4::new(-9.10, 11.12, -13.14, 15.16);

    assert_eq!(a.snapped(b), a.as_inner().snapped(b));
}
