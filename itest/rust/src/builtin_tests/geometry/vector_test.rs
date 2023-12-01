/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::framework::itest;

use godot::builtin::inner::{InnerVector2, InnerVector3};
use godot::builtin::{real, real_consts::PI, Vector2, Vector3};
use godot::private::class_macros::assert_eq_approx;

#[itest]
fn vector2_equiv() {
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

#[itest]
fn vector3_equiv() {
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
