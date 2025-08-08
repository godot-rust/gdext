/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::builtin::{real, Vector4Axis, Vector4i};

use crate::framework::itest;

#[itest]
fn abs() {
    let a = Vector4i::new(-1, 2, -3, 4);

    assert_eq!(a.abs(), a.as_inner().abs());
}

#[itest]
fn clamp() {
    let a = Vector4i::new(12, -34, 56, -78);

    let min = Vector4i::new(15, 15, 15, 15);
    let max = Vector4i::new(30, 30, 30, 30);

    assert_eq!(a.clamp(min, max), a.as_inner().clamp(min, max));
}

#[itest]
fn length() {
    let a = Vector4i::new(1, 3, 4, 5);

    assert_eq!(a.length(), a.as_inner().length() as real);
}

#[itest]
fn length_squared() {
    let a = Vector4i::new(1, 3, 4, 5);

    assert_eq!(a.length_squared(), a.as_inner().length_squared() as i32);
}

#[itest]
fn max_axis() {
    let a = Vector4i::new(10, 5, 0, -5);
    let b = Vector4i::new(10, 10, 10, 10);

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
    let a = Vector4i::new(10, 5, 0, -5);
    let b = Vector4i::new(10, 10, 10, 10);

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
fn sign() {
    let a = Vector4i::new(-1, 2, -3, 4);
    let b = Vector4i::new(-0, 0, -0, 0);

    assert_eq!(a.sign(), a.as_inner().sign());
    assert_eq!(b.sign(), b.as_inner().sign());
}

#[itest]
fn snapped() {
    let a = Vector4i::new(12, 34, 56, -78);
    let b = Vector4i::new(5, -5, 6, 6);
    let c = Vector4i::new(0, 3, 0, 0);
    let d = Vector4i::new(3, 0, -3, 0);

    assert_eq!(a.snapped(b), a.as_inner().snapped(b));
    assert_eq!(c.snapped(d), c.as_inner().snapped(d));
}
