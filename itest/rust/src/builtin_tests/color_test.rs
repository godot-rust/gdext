/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::builtin::math::assert_eq_approx;
use godot::builtin::{Color, ColorChannelOrder, ColorHsv};

use crate::framework::itest;

#[itest]
fn color_from_rgba8() {
    assert_eq!(
        Color::from_rgba8(0x00, 0x01, 0xff, 0x80),
        Color::from_rgba(0.0, 1.0 / 255.0, 1.0, 128.0 / 255.0)
    );
}

#[itest]
fn color_from_u32() {
    const D: f32 = 255.0;
    assert_eq!(
        Color::from_u32_rgba(0x01020304, ColorChannelOrder::RGBA),
        Color::from_rgba(1.0 / D, 2.0 / D, 3.0 / D, 4.0 / D)
    );
    assert_eq!(
        Color::from_u32_rgba(0x01020304, ColorChannelOrder::ABGR),
        Color::from_rgba(4.0 / D, 3.0 / D, 2.0 / D, 1.0 / D)
    );
    assert_eq!(
        Color::from_u32_rgba(0x01020304, ColorChannelOrder::ARGB),
        Color::from_rgba(2.0 / D, 3.0 / D, 4.0 / D, 1.0 / D)
    );
}

#[itest]
fn color_from_u64() {
    const D: f32 = 65535.0;
    assert_eq!(
        Color::from_u64_rgba(0x0001000200030004, ColorChannelOrder::RGBA),
        Color::from_rgba(1.0 / D, 2.0 / D, 3.0 / D, 4.0 / D)
    );
    assert_eq!(
        Color::from_u64_rgba(0x0001000200030004, ColorChannelOrder::ABGR),
        Color::from_rgba(4.0 / D, 3.0 / D, 2.0 / D, 1.0 / D)
    );
    assert_eq!(
        Color::from_u64_rgba(0x0001000200030004, ColorChannelOrder::ARGB),
        Color::from_rgba(2.0 / D, 3.0 / D, 4.0 / D, 1.0 / D)
    );
}

#[itest]
fn color_from_html() {
    assert_eq!(
        Color::from_string("#abcd"),
        Some(Color::from_rgba8(0xaa, 0xbb, 0xcc, 0xdd))
    );
    assert_eq!(Color::from_string("#abcde"), None);
    assert_eq!(Color::from_string("#abcg"), None);
}

#[itest]
fn color_from_string() {
    // We don't test all possibilities because internally the string is just passed to the engine.
    assert_eq!(
        Color::from_string("white"),
        Some(Color::from_rgba(1.0, 1.0, 1.0, 1.0))
    );
    assert_eq!(
        Color::from_string("#abcd"),
        Some(Color::from_rgba8(0xaa, 0xbb, 0xcc, 0xdd))
    );
    assert_eq!(Color::from_string("#abcde"), None);
    assert_eq!(Color::from_string(""), None);
    assert_eq!(Color::from_string("octarine"), None); // Sorry, Rincewind.
}

#[itest]
fn color_get_set_u8() {
    let mut c = Color::default();
    for i in 0..=255 {
        c.set_r8(i);
        assert_eq!(c.r8(), i);
    }
}

#[itest]
fn color_blend() {
    // Just to check the argument order: which color is blended over which one?
    assert_eq!(
        Color::from_html("#ff0000ff")
            .unwrap()
            .blend(Color::from_html("#00ff00ff").unwrap()),
        Color::from_html("#00ff00ff").unwrap()
    );
}

#[itest]
fn color_to_u32() {
    let c = Color::from_html("#01020304").unwrap();
    assert_eq!(c.to_u32(ColorChannelOrder::RGBA), 0x01020304);
    assert_eq!(c.to_u32(ColorChannelOrder::ABGR), 0x04030201);
    assert_eq!(c.to_u32(ColorChannelOrder::ARGB), 0x04010203);
}

#[itest]
fn color_to_u64() {
    let c = Color::from_html("#01020304").unwrap();
    assert_eq!(c.to_u64(ColorChannelOrder::RGBA), 0x0101_0202_0303_0404);
    assert_eq!(c.to_u64(ColorChannelOrder::ABGR), 0x0404_0303_0202_0101);
    assert_eq!(c.to_u64(ColorChannelOrder::ARGB), 0x0404_0101_0202_0303);
}

// Multiple specific cases because HSV->RGB conversion algorithm used is very dependent on Hue value, taking into account different values
// based on the Hue sector.
const COLOR_HSV_CASES_HSV: [(f32, f32, f32); 9] = [
    (0.0, 0.43, 1.),
    (0.05, 0.34, 0.23),
    (0.22, 1., 0.54),
    (0.42, 0.42, 0.15),
    (0.58, 0.89, 0.23),
    (0.74, 0.69, 0.18),
    (0.96, 0.82, 0.12),
    (1., 0.23, 0.73),
    (1., 0.55, 0.23),
];

const COLOR_HSV_CASES_RGB: [(f32, f32, f32); 6] = [
    (0.32, 0.75, 1.),
    (1., 0.32, 1.),
    (0.5, 0., 0.),
    (0.23, 0.12, 0.78),
    (1., 1., 1.),
    (0., 0., 0.),
];

#[itest]
fn color_from_color_hsv() {
    for (h, s, v) in COLOR_HSV_CASES_HSV {
        let c_hsv = ColorHsv::from_hsv(h, s, v);
        let c1 = Color::from_hsv(c_hsv.h as f64, c_hsv.s as f64, c_hsv.v as f64);
        let c2: Color = c_hsv.to_rgb();

        assert_eq_approx!(c1, c2, "h: {h}, s: {s}, v: {v}");
    }
}

#[itest]
fn color_hsv_wraps_correctly() {
    for (hue_origin, hue_shift, hue_expected) in [
        (0.75, -0.85, 0.90),
        (0.5, 0.65, 0.15),
        (0.15, 3.25, 0.40),
        (0.45, -5.43, 0.02),
    ] {
        let mut c_hsv = ColorHsv {
            h: hue_origin,
            ..Default::default()
        };
        c_hsv.h += hue_shift;

        c_hsv = c_hsv.normalized_wrapped_h();

        assert_eq_approx!(c_hsv.h, hue_expected);
    }
}

#[itest]
fn color_hsv_roundtrip() {
    for (h, s, v) in COLOR_HSV_CASES_HSV {
        let c1 = ColorHsv::from_hsv(h, s, v);
        let c2 = c1.to_rgb().to_hsv();

        assert_eq_approx!(c1, c2, "h: {h}, s: {s}, v: {v}");
    }
}

#[itest]
fn color_hsv_from_color_roundtrip() {
    for (r, g, b) in COLOR_HSV_CASES_RGB {
        let c = Color::from_rgb(r, g, b);
        let c_back = c.to_hsv().to_rgb();

        assert_eq_approx!(c, c_back);
    }
}

#[itest]
fn color_hsv_multi_roundtrip() {
    for (r, g, b) in COLOR_HSV_CASES_RGB {
        let original = Color::from_rgb(r, g, b);
        let mut c_back = original;
        for _ in 0..10 {
            c_back = c_back.to_hsv().to_rgb();
        }
        assert_eq_approx!(original, c_back);
    }
}

// Check that color constants match their Godot value exactly.
//
// Occasionally, this can be manually cross-checked against extension_api.json. We currently don't codegen those constants, and the values
// there are in float, so may not match exactly.
#[itest]
fn color_constants() {
    for (name, rust_color) in Color::ALL_GODOT_COLORS.iter().copied() {
        let godot_color = Color::from_string(name)
            .unwrap_or_else(|| panic!("Color constant {name} not found in Godot"));

        assert_eq!(rust_color, godot_color, "Color mismatch for {name}");
    }
}
