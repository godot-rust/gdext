/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::hint::black_box;

use godot::builtin::{Color, ColorHsv};

use crate::framework::bench;

#[bench]
fn godot_from_hsv() -> Color {
    Color::from_hsv(black_box(0.23), black_box(0.54), black_box(0.75))
}

#[bench]
fn rust_from_hsv() -> Color {
    ColorHsv::from_hsv(black_box(0.23), black_box(0.54), black_box(0.75)).to_rgb()
}

#[bench]
fn color_hsv_hue_wrap() -> ColorHsv {
    ColorHsv {
        h: 1.15,
        ..Default::default()
    }
    .normalized_wrapped_h()
}

#[bench]
fn rgb_to_hsv_roundtrip() -> Color {
    let color = Color {
        r: 0.23,
        g: 0.56,
        b: 0.93,
        a: 1.,
    };

    color.to_hsv().to_rgb()
}

#[bench]
fn rgb_to_hsv_mutate_roundtrip() -> Color {
    let color = Color {
        r: 0.23,
        g: 0.56,
        b: 0.93,
        a: 1.,
    };

    let mut hsv = color.to_hsv();

    hsv.h += 0.15;
    hsv.s = 2.0;
    hsv.v += 0.10;
    hsv = hsv.normalized_wrapped_h();

    hsv.to_rgb()
}
