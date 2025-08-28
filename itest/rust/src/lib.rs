/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::sync::Mutex;

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

static LEVELS_SEEN: Mutex<Vec<InitLevel>> = Mutex::new(Vec::new());

#[gdextension(entry_symbol = itest_init)]
unsafe impl ExtensionLibrary for framework::IntegrationTests {
    fn min_level() -> InitLevel {
        InitLevel::Core
    }
    fn on_level_init(level: InitLevel) {
        LEVELS_SEEN.lock().unwrap().push(level);
        match level {
            InitLevel::Core => {
                // make sure we can access early core singletons
                object_tests::test_early_core_singletons();
            }
            InitLevel::Servers => {}
            InitLevel::Scene => {}
            InitLevel::Editor => {}
        }
    }
}

// Ensure that we saw all the init levels expected.
#[crate::framework::itest]
fn observed_all_init_levels() {
    let levels_seen = LEVELS_SEEN.lock().unwrap().clone();
    assert_eq!(levels_seen[0], InitLevel::Core);
    assert_eq!(levels_seen[1], InitLevel::Servers);
    assert_eq!(levels_seen[2], InitLevel::Scene);
    // NOTE: some tests don't see editor mode
    if let Some(level_3) = levels_seen.get(3) {
        assert_eq!(*level_3, InitLevel::Editor);
    }
}
