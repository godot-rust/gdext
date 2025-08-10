/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::sync::atomic::{AtomicI32, Ordering};

use godot::prelude::*;

static SUCCESSFUL_CALLS: AtomicI32 = AtomicI32::new(0);

#[derive(GodotClass)]
#[class(init)]
struct ConversionTest {}

#[godot_api]
impl ConversionTest {
    #[func]
    fn accept_i32(value: i32) -> String {
        SUCCESSFUL_CALLS.fetch_add(1, Ordering::SeqCst);
        value.to_string()
    }

    #[func]
    fn accept_f32(value: f32) -> String {
        SUCCESSFUL_CALLS.fetch_add(1, Ordering::SeqCst);
        value.to_string()
    }

    #[func]
    fn return_i32() -> i32 {
        SUCCESSFUL_CALLS.fetch_add(1, Ordering::SeqCst);
        123
    }

    #[func]
    fn return_f32() -> f32 {
        SUCCESSFUL_CALLS.fetch_add(1, Ordering::SeqCst);
        123.45
    }

    #[func]
    fn successful_calls() -> i32 {
        SUCCESSFUL_CALLS.load(Ordering::SeqCst)
    }
}
