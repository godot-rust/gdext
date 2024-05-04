/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::prelude::*;

#[derive(GodotClass)]
#[class(init)]
struct ConversionTest {}

#[godot_api]
impl ConversionTest {
    #[func]
    fn accept_i32(value: i32) -> String {
        value.to_string()
    }

    #[func]
    fn accept_f32(value: f32) -> String {
        value.to_string()
    }

    #[func]
    fn return_i32() -> i32 {
        123
    }

    #[func]
    fn return_f32() -> f32 {
        123.45
    }
}
