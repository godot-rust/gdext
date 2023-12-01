/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::framework::itest;
use godot::builtin::{Color, ColorChannelOrder};

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
        Color::from_u32_rgba(0x01020304, ColorChannelOrder::Rgba),
        Color::from_rgba(1.0 / D, 2.0 / D, 3.0 / D, 4.0 / D)
    );
    assert_eq!(
        Color::from_u32_rgba(0x01020304, ColorChannelOrder::Abgr),
        Color::from_rgba(4.0 / D, 3.0 / D, 2.0 / D, 1.0 / D)
    );
    assert_eq!(
        Color::from_u32_rgba(0x01020304, ColorChannelOrder::Argb),
        Color::from_rgba(2.0 / D, 3.0 / D, 4.0 / D, 1.0 / D)
    );
}

#[itest]
fn color_from_u64() {
    const D: f32 = 65535.0;
    assert_eq!(
        Color::from_u64_rgba(0x0001000200030004, ColorChannelOrder::Rgba),
        Color::from_rgba(1.0 / D, 2.0 / D, 3.0 / D, 4.0 / D)
    );
    assert_eq!(
        Color::from_u64_rgba(0x0001000200030004, ColorChannelOrder::Abgr),
        Color::from_rgba(4.0 / D, 3.0 / D, 2.0 / D, 1.0 / D)
    );
    assert_eq!(
        Color::from_u64_rgba(0x0001000200030004, ColorChannelOrder::Argb),
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
    assert_eq!(c.to_u32(ColorChannelOrder::Rgba), 0x01020304);
    assert_eq!(c.to_u32(ColorChannelOrder::Abgr), 0x04030201);
    assert_eq!(c.to_u32(ColorChannelOrder::Argb), 0x04010203);
}

#[itest]
fn color_to_u64() {
    let c = Color::from_html("#01020304").unwrap();
    assert_eq!(c.to_u64(ColorChannelOrder::Rgba), 0x0101_0202_0303_0404);
    assert_eq!(c.to_u64(ColorChannelOrder::Abgr), 0x0404_0303_0202_0101);
    assert_eq!(c.to_u64(ColorChannelOrder::Argb), 0x0404_0101_0202_0303);
}
