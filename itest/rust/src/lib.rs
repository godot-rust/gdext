/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::init::{gdextension, ExtensionLibrary, InitLevel, InitStage};

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

    fn on_stage_init(stage: InitStage) {
        object_tests::on_stage_init(stage);
    }

    fn on_stage_deinit(stage: InitStage) {
        object_tests::on_stage_deinit(stage);
    }

    #[cfg(since_api = "4.5")]
    fn on_main_loop_frame() {
        object_tests::on_main_loop_frame();
    }
}
