/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::builtin::{real, Vector3Axis, Vector3i};

use crate::framework::itest;

#[itest]
fn abs() {
    let a = Vector3i::new(-1, 2, -3);

    assert_eq!(a.abs(), a.as_inner().abs());
}

#[itest]
fn clamp() {
    let a = Vector3i::new(12, -34, 56);

    let min = Vector3i::new(15, 15, 15);
    let max = Vector3i::new(30, 30, 30);

    assert_eq!(a.clamp(min, max), a.as_inner().clamp(min, max));
}

#[itest]
fn length() {
    let a = Vector3i::new(1, 2, 3);

    assert_eq!(a.length(), a.as_inner().length() as real);
}

#[itest]
fn length_squared() {
    let a = Vector3i::new(1, 2, 3);

    assert_eq!(a.length_squared() as i64, a.as_inner().length_squared());
}

#[itest]
fn max_axis() {
    let a = Vector3i::new(10, 5, 0);
    let b = Vector3i::new(10, 10, 10);

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
    let a = Vector3i::new(10, 5, 0);
    let b = Vector3i::new(10, 10, 10);

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
fn sign() {
    let a = Vector3i::new(-1, 2, -3);
    let b = Vector3i::new(-0, 0, -0);

    assert_eq!(a.sign(), a.as_inner().sign());
    assert_eq!(b.sign(), b.as_inner().sign());
}

#[itest]
fn snapped() {
    let a = Vector3i::new(12, 34, -56);
    let b = Vector3i::new(5, -5, 6);
    let c = Vector3i::new(0, 3, 0);
    let d = Vector3i::new(3, 0, 0);

    assert_eq!(a.snapped(b), a.as_inner().snapped(b));
    assert_eq!(c.snapped(d), c.as_inner().snapped(d));
}
