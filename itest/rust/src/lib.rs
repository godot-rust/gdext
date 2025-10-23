#![cfg_attr(published_docs, feature(doc_cfg))]
/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::init::{gdextension, ExtensionLibrary, InitLevel};

mod benchmarks;
mod builtin_tests;
mod common;
mod engine_tests;
mod framework;
mod object_tests;
mod register_tests;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Entry point

#[gdextension(entry_symbol = itest_init)]
unsafe impl ExtensionLibrary for framework::IntegrationTests {
    fn min_level() -> InitLevel {
        InitLevel::Core
    }

    fn on_level_init(level: InitLevel) {
        object_tests::on_level_init(level);
    }

    #[cfg(since_api = "4.5")]
    fn on_main_loop_startup() {
        object_tests::on_main_loop_startup();
    }

    #[cfg(since_api = "4.5")]
    fn on_main_loop_frame() {
        object_tests::on_main_loop_frame();
    }

    #[cfg(since_api = "4.5")]
    fn on_main_loop_shutdown() {
        object_tests::on_main_loop_shutdown();
    }
}
