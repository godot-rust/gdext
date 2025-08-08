/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::builtin::math::assert_eq_approx;
use godot::builtin::{Quaternion, Vector3};

use crate::framework::{expect_panic, itest};

#[itest]
fn quaternion_default() {
    let quat = Quaternion::default();

    assert_eq!(quat.x, 0.0);
    assert_eq!(quat.y, 0.0);
    assert_eq!(quat.z, 0.0);
    assert_eq!(quat.w, 1.0);
}

#[itest]
fn quaternion_from_xyzw() {
    let quat = Quaternion::new(0.2391, 0.099, 0.3696, 0.8924);

    assert_eq!(quat.x, 0.2391);
    assert_eq!(quat.y, 0.099);
    assert_eq!(quat.z, 0.3696);
    assert_eq!(quat.w, 0.8924);
}

#[itest]
fn quaternion_from_axis_angle() {
    // 1. Should generate quaternion from axis angle.
    let quat = Quaternion::from_axis_angle(Vector3::BACK, 1.0);

    // Taken from doing this in GDScript.
    assert_eq!(quat.x, 0.0);
    assert_eq!(quat.y, 0.0);
    assert_eq_approx!(quat.z, 0.479426);
    assert_eq_approx!(quat.w, 0.877583);

    // 2. Should panic if axis is not normalized.
    expect_panic("Quaternion axis (0, 0, 0) is not normalized.", || {
        Quaternion::from_axis_angle(Vector3::ZERO, 1.0);
    });

    expect_panic("Quaternion axis (0, 0.7, 0) is not normalized.", || {
        Quaternion::from_axis_angle(Vector3::UP * 0.7, 1.0);
    });
}

#[itest]
fn quaternion_normalization() {
    // 1. Should panic on quaternions with length 0.
    expect_panic("Quaternion has length 0", || {
        Quaternion::new(0.0, 0.0, 0.0, 0.0).normalized();
    });

    // 2. Should not panic on any other length.
    let quat = Quaternion::default().normalized();
    assert_eq!(quat.length(), 1.0);
    assert!(quat.is_normalized());
}

#[itest]
fn quaternion_slerp() {
    let a = Quaternion::new(-1.0, -1.0, -1.0, 10.0);
    let b = Quaternion::new(3.0, 3.0, 3.0, 5.0);

    // 1. Should perform interpolation.
    let outcome = a.normalized().slerp(b.normalized(), 1.0);
    let expected = Quaternion::new(0.41602516, 0.41602516, 0.41602516, 0.69337523);
    assert_eq_approx!(outcome, expected);

    // 2. Should panic on quaternions that are not normalized.
    expect_panic("Slerp requires normalized quaternions", || {
        a.slerp(b, 1.9);
    });

    // 3. Should not panic on default values.
    let outcome = Quaternion::default().slerp(Quaternion::default(), 1.0);
    assert_eq!(outcome, Quaternion::default());
}

#[itest]
fn quaternion_slerpni() {
    let a = Quaternion::new(-1.0, -1.0, -1.0, 10.0);
    let b = Quaternion::new(3.0, 3.0, 3.0, 6.0);

    // 1. Should perform interpolation.
    let outcome = a.normalized().slerpni(b.normalized(), 1.0);
    let expected = Quaternion::new(0.37796447, 0.37796447, 0.37796447, 0.75592893);
    assert_eq_approx!(outcome, expected);

    // 2. Should panic on quaternions that are not normalized.
    expect_panic("Slerpni requires normalized quaternions", || {
        a.slerpni(b, 1.9);
    });

    // 3. Should not panic on default values.
    let outcome = Quaternion::default().slerpni(Quaternion::default(), 1.0);
    assert_eq!(outcome, Quaternion::default());
}

#[itest]
fn quaternion_spherical_cubic_interpolate() {
    let pre_a = Quaternion::new(-1.0, -1.0, -1.0, -1.0);
    let a = Quaternion::new(0.0, 0.0, 0.0, 1.0);
    let b = Quaternion::new(0.0, 1.0, 0.0, 2.0);
    let post_b = Quaternion::new(2.0, 2.0, 2.0, 2.0);

    // 1. Should perform interpolation.
    let outcome =
        a.spherical_cubic_interpolate(b.normalized(), pre_a.normalized(), post_b.normalized(), 0.5);

    // Taken from doing this in GDScript.
    let expected = Quaternion::new(-0.072151, 0.176298, -0.072151, 0.979034);
    assert_eq_approx!(outcome, expected);

    // 2. Should panic on quaternions that are not normalized.
    expect_panic(
        "Spherical cubic interpolation requires normalized quaternions",
        || {
            a.spherical_cubic_interpolate(b, pre_a, post_b, 0.5);
        },
    );

    // 3. Should not panic on default returns when inputs are normalized.
    let outcome = Quaternion::default().spherical_cubic_interpolate(
        Quaternion::default(),
        Quaternion::default(),
        Quaternion::default(),
        1.0,
    );
    assert_eq!(outcome, Quaternion::default());
}

#[itest]
fn quaternion_spherical_cubic_interpolate_in_time() {
    let pre_a = Quaternion::new(-1.0, -1.0, -1.0, -1.0);
    let a = Quaternion::new(0.0, 0.0, 0.0, 1.0);
    let b = Quaternion::new(0.0, 1.0, 0.0, 2.0);
    let post_b = Quaternion::new(2.0, 2.0, 2.0, 2.0);

    // 1. Should perform interpolation.
    let outcome = a.spherical_cubic_interpolate_in_time(
        b.normalized(),
        pre_a.normalized(),
        post_b.normalized(),
        0.5,
        0.1,
        0.1,
        0.1,
    );

    // Taken from doing this in GDScript.
    let expected = Quaternion::new(0.280511, 0.355936, 0.280511, 0.84613);
    assert_eq_approx!(outcome, expected);

    // 2. Should panic on quaternions that are not normalized.
    expect_panic(
        "Spherical cubic interpolation in time requires normalized quaternions",
        || {
            a.spherical_cubic_interpolate_in_time(b, pre_a, post_b, 0.5, 0.1, 0.1, 0.1);
        },
    );

    // 3. Should not panic on default returns when inputs are normalized.
    let outcome = Quaternion::default().spherical_cubic_interpolate_in_time(
        Quaternion::default(),
        Quaternion::default(),
        Quaternion::default(),
        1.0,
        1.0,
        1.0,
        1.0,
    );
    assert_eq!(outcome, Quaternion::default())
}

#[itest]
fn quaternion_mul1() {
    use std::f32::consts::PI;

    use godot::builtin::real;

    let q = Quaternion::from_axis_angle(Vector3::UP, (PI / 2.0) as real);
    let rotated = q * Vector3::new(1.0, 4.2, 0.0);
    assert_eq_approx!(rotated.x, 0.0);
    assert_eq_approx!(rotated.y, 4.2);
    assert_eq_approx!(rotated.z, -1.0);
}

#[itest]
fn quaternion_mul2() {
    use std::f32::consts::PI;

    use godot::builtin::real;

    let q = Quaternion::from_axis_angle(-Vector3::UP, (PI / 2.0) as real);
    let rotated = q * Vector3::new(1.0, 4.2, 2.0);
    assert_eq_approx!(rotated.x, -2.0);
    assert_eq_approx!(rotated.y, 4.2);
    assert_eq_approx!(rotated.z, 1.0);
}

#[itest]
fn quaternion_mul3() {
    use std::f32::consts::PI;

    use godot::builtin::real;

    let q = Quaternion::from_axis_angle(Vector3::UP, (-PI * 3.0 / 4.0) as real);
    let rotated = q * Vector3::new(1.0, 3.0, 5.0);
    assert_eq_approx!(rotated.x, -4.2426405);
    assert_eq_approx!(rotated.y, 3.0);
    assert_eq_approx!(rotated.z, -2.828427);
}
// TODO more tests
