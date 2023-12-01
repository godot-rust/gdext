/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::engine::{Engine, Node};
use godot::obj::Gd;
use godot::sys;
use std::collections::HashSet;

mod bencher;
mod runner;

pub use bencher::*;
pub use runner::*;

/// Allow re-import as `crate::framework::itest`.
pub use godot::test::{bench, itest};

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Plugin registration

// Registers all the `#[itest]` tests and `#[bench]` benchmarks.
sys::plugin_registry!(pub(crate) __GODOT_ITEST: RustTestCase);
sys::plugin_registry!(pub(crate) __GODOT_BENCH: RustBenchmark);

/// Finds all `#[itest]` tests.
fn collect_rust_tests(filters: &[String]) -> (Vec<RustTestCase>, usize, bool) {
    let mut all_files = HashSet::new();
    let mut tests: Vec<RustTestCase> = vec![];
    let mut is_focus_run = false;

    sys::plugin_foreach!(__GODOT_ITEST; |test: &RustTestCase| {
        // First time a focused test is encountered, switch to "focused" mode and throw everything away.
        if !is_focus_run && test.focused {
            tests.clear();
            all_files.clear();
            is_focus_run = true;
        }

        // Only collect tests if normal mode, or focus mode and test is focused.
        if (!is_focus_run || test.focused) && passes_filter(filters, test.name) {
            all_files.insert(test.file);
            tests.push(*test);
        }
    });

    // Sort alphabetically for deterministic run order
    tests.sort_by_key(|test| test.file);

    (tests, all_files.len(), is_focus_run)
}

/// Finds all `#[bench]` benchmarks.
fn collect_rust_benchmarks() -> (Vec<RustBenchmark>, usize) {
    let mut all_files = HashSet::new();
    let mut benchmarks: Vec<RustBenchmark> = vec![];

    sys::plugin_foreach!(__GODOT_BENCH; |bench: &RustBenchmark| {
        benchmarks.push(*bench);
        all_files.insert(bench.file);
    });

    // Sort alphabetically for deterministic run order
    benchmarks.sort_by_key(|bench| bench.file);

    (benchmarks, all_files.len())
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Shared types

pub struct TestContext {
    pub scene_tree: Gd<Node>,
    pub property_tests: Gd<Node>,
}

#[derive(Copy, Clone)]
pub struct RustTestCase {
    pub name: &'static str,
    pub file: &'static str,
    pub skipped: bool,
    /// If one or more tests are focused, only they will be executed. Helpful for debugging and working on specific features.
    pub focused: bool,
    #[allow(dead_code)]
    pub line: u32,
    pub function: fn(&TestContext),
}

#[derive(Copy, Clone)]
pub struct RustBenchmark {
    pub name: &'static str,
    pub file: &'static str,
    #[allow(dead_code)]
    pub line: u32,
    pub function: fn(),
    pub repetitions: usize,
}

pub fn passes_filter(filters: &[String], test_name: &str) -> bool {
    filters.is_empty() || filters.iter().any(|x| test_name.contains(x))
}

pub fn expect_panic(context: &str, code: impl FnOnce() + std::panic::UnwindSafe) {
    use std::panic;

    // Exchange panic hook, to disable printing during expected panics
    let prev_hook = panic::take_hook();
    panic::set_hook(Box::new(|_panic_info| {}));

    // Run code that should panic, restore hook
    let panic = panic::catch_unwind(code);
    panic::set_hook(prev_hook);

    assert!(
        panic.is_err(),
        "code should have panicked but did not: {context}",
    );
}

/// Disable printing errors from Godot. Ideally we should catch and handle errors, ensuring they happen when
/// expected. But that isn't possible, so for now we can just disable printing the error to avoid spamming
/// the terminal when tests should error.
pub fn suppress_godot_print(mut f: impl FnMut()) {
    Engine::singleton().set_print_error_messages(false);
    f();
    Engine::singleton().set_print_error_messages(true);
}
