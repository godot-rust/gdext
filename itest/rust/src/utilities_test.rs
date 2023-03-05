/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::itest;

use godot::builtin::Variant;
use godot::engine::utilities::*;

#[itest]
fn utilities_abs() {
    let input = Variant::from(-7);
    let output = abs(input);

    assert_eq!(output, Variant::from(7));
}

#[itest]
fn utilities_sign() {
    let input = Variant::from(-7);
    let output = sign(input);

    assert_eq!(output, Variant::from(-1));
}

#[itest]
fn utilities_wrap() {
    let output = wrap(Variant::from(3.4), Variant::from(2.0), Variant::from(3.0));
    assert_eq!(output, Variant::from(2.4));

    let output = wrap(
        Variant::from(-5.7),
        Variant::from(-3.0),
        Variant::from(-2.0),
    );
    assert_eq!(output, Variant::from(-2.7));
}

#[itest]
fn utilities_max() {
    let output = max(
        Variant::from(1.0),
        Variant::from(3.0),
        &[Variant::from(5.0), Variant::from(7.0)],
    );
    assert_eq!(output, Variant::from(7.0));

    let output = max(
        Variant::from(-1.0),
        Variant::from(-3.0),
        &[Variant::from(-5.0), Variant::from(-7.0)],
    );
    assert_eq!(output, Variant::from(-1.0));
}
