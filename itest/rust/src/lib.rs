/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::init::{gdextension, ExtensionLibrary};
use godot::sys;

mod array_test;
mod base_test;
mod basis_test;
mod builtin_test;
mod codegen_test;
mod color_test;
mod dictionary_test;
mod enum_test;
mod export_test;
mod gdscript_ffi_test;
mod node_test;
mod object_test;
mod packed_array_test;
mod quaternion_test;
mod singleton_test;
mod string_test;
mod utilities_test;
mod variant_test;
mod virtual_methods_test;

mod runner;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// API for test cases

use godot::test::itest;

pub(crate) fn expect_panic(context: &str, code: impl FnOnce() + std::panic::UnwindSafe) {
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

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Entry point + #[itest] test registration

#[gdextension(entry_point=itest_init)]
unsafe impl ExtensionLibrary for runner::IntegrationTests {}

// Registers all the `#[itest]` tests.
sys::plugin_registry!(__GODOT_ITEST: RustTestCase);

/// Finds all `#[itest]` tests.
fn collect_rust_tests() -> (Vec<RustTestCase>, usize) {
    let mut all_files = std::collections::HashSet::new();
    let mut tests: Vec<RustTestCase> = vec![];

    sys::plugin_foreach!(__GODOT_ITEST; |test: &RustTestCase| {
        all_files.insert(test.file);
        tests.push(*test);
    });

    // Sort alphabetically for deterministic run order
    tests.sort_by_key(|test| test.file);

    (tests, all_files.len())
}

#[derive(Copy, Clone)]
struct RustTestCase {
    name: &'static str,
    file: &'static str,
    skipped: bool,
    #[allow(dead_code)]
    line: u32,
    function: fn(),
}
