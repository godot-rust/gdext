/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::builtin::{real, Vector2Axis, Vector2i};

use crate::framework::itest;

#[itest]
fn abs() {
    let a = Vector2i::new(-1, 2);

    assert_eq!(a.abs(), a.as_inner().abs());
}

#[itest]
fn aspect() {
    let a = Vector2i::new(4, 2);

    assert_eq!(a.aspect(), a.as_inner().aspect() as real);
}

#[itest]
fn clamp() {
    let a = Vector2i::new(12, 34);

    let min = Vector2i::new(15, 15);
    let max = Vector2i::new(30, 30);

    assert_eq!(a.clamp(min, max), a.as_inner().clamp(min, max));
}

#[itest]
fn length() {
    let a = Vector2i::new(3, 4);

    assert_eq!(a.length(), a.as_inner().length() as real);
}

#[itest]
fn length_squared() {
    let a = Vector2i::new(3, 4);

    assert_eq!(a.length_squared(), a.as_inner().length_squared() as i32);
}

#[itest]
fn max_axis() {
    let a = Vector2i::new(10, 5);
    let b = Vector2i::new(10, 10);

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
    let a = Vector2i::new(10, 5);
    let b = Vector2i::new(10, 10);

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
fn sign() {
    let a = Vector2i::new(-1, 2);
    let b = Vector2i::new(-0, 0);

    assert_eq!(a.sign(), a.as_inner().sign());
    assert_eq!(b.sign(), b.as_inner().sign());
}

#[itest]
fn snapped() {
    let a = Vector2i::new(12, 34);
    let b = Vector2i::new(5, -5);
    let c = Vector2i::new(0, 0);
    let d = Vector2i::new(3, 0);

    assert_eq!(a.snapped(b), a.as_inner().snapped(b));
    assert_eq!(c.snapped(d), c.as_inner().snapped(d));
}
