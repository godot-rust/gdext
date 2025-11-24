/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// Note: in the past, we had is_instance_valid() -- we also tested that godot-rust is not susceptible to the godot-cpp issue
// https://github.com/godotengine/godot-cpp/issues/1390.

use godot::builtin::{vslice, GString, Variant};
use godot::global::*;

use crate::framework::itest;

#[itest]
fn utilities_abs() {
    let input = Variant::from(-7);
    let output = abs(&input);

    assert_eq!(output, Variant::from(7));
}

#[itest]
fn utilities_sign() {
    let input = Variant::from(-7);
    let output = sign(&input);

    assert_eq!(output, Variant::from(-1));
}

#[itest]
fn utilities_str() {
    let a = 12;
    let b = " is a ";
    let c = true;
    let d = " number";
    let concat = str(vslice![a, b, c, d]);

    let empty = str(&[]);

    assert_eq!(concat, "12 is a true number");
    assert_eq!(concat, godot_str!("{a}{b}{c}{d}"));
    assert_eq!(empty, GString::new());
}

#[itest]
fn utilities_wrap() {
    let output = wrap(
        &Variant::from(3.4),
        &Variant::from(2.0),
        &Variant::from(3.0),
    );
    assert_eq!(output, Variant::from(2.4));

    let output = wrap(
        &Variant::from(-5.7),
        &Variant::from(-3.0),
        &Variant::from(-2.0),
    );
    assert_eq!(output, Variant::from(-2.7));
}

#[itest]
fn utilities_max() {
    let output = max(&Variant::from(1.0), &Variant::from(3.0), vslice![5.0, 7.0]);
    assert_eq!(output, Variant::from(7.0));

    let output = max(
        &Variant::from(-1.0),
        &Variant::from(-3.0),
        vslice![-5.0, -7.0],
    );
    assert_eq!(output, Variant::from(-1.0));
}
