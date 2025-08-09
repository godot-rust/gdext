/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::sync::atomic::{AtomicBool, Ordering};

use godot::init::InitLevel;
use godot::obj::NewAlloc;
use godot::register::{godot_api, GodotClass};

use crate::framework::itest;

static HAS_RUN: AtomicBool = AtomicBool::new(false);

#[derive(GodotClass)]
#[class(base = Object, init)]
struct SomeObject {}

#[godot_api]
impl SomeObject {
    #[func]
    pub fn set_has_run_true(&self) {
        HAS_RUN.store(true, Ordering::Release);
    }
}

// Run during on the `on_level_init` of the entry point.
pub fn initialize_init_level_test(level: InitLevel) {
    if level == InitLevel::Servers {
        assert!(!HAS_RUN.load(Ordering::Acquire));

        let mut some_object = SomeObject::new_alloc();
        // Need to go through Godot here as otherwise we bypass the failure.
        some_object.call("set_has_run_true", &[]);
        some_object.free();
    }
}

// Ensure that the above function actually ran.
#[itest]
fn class_run_during_servers_init() {
    assert!(HAS_RUN.load(Ordering::Acquire));
}
