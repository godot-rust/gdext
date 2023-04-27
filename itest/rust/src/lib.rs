/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::engine::{Engine, Node};
use godot::init::{gdextension, ExtensionLibrary};
use godot::obj::Gd;
use godot::sys;

mod array_test;
mod base_test;
mod basis_test;
mod builtin_test;
mod callable_test;
mod codegen_test;
mod color_test;
mod derive_variant;
mod dictionary_test;
mod enum_test;
mod export_test;
mod gdscript_ffi_test;
mod init_test;
mod native_structures_test;
mod node_test;
mod object_test;
mod option_ffi_test;
mod packed_array_test;
mod projection_test;
mod quaternion_test;
mod rect2i_test;
mod rid_test;
mod singleton_test;
mod string;
mod transform2d_test;
mod transform3d_test;
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

/// Disable printing errors from Godot. Ideally we should catch and handle errors, ensuring they happen when
/// expected. But that isn't possible, so for now we can just disable printing the error to avoid spamming
/// the terminal when tests should error.
pub(crate) fn suppress_godot_print(mut f: impl FnMut()) {
    Engine::singleton().set_print_error_messages(false);
    f();
    Engine::singleton().set_print_error_messages(true);
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Entry point + #[itest] test registration

#[gdextension(entry_point=itest_init)]
unsafe impl ExtensionLibrary for runner::IntegrationTests {}

// Registers all the `#[itest]` tests.
sys::plugin_registry!(__GODOT_ITEST: RustTestCase);

/// Finds all `#[itest]` tests.
fn collect_rust_tests() -> (Vec<RustTestCase>, usize, bool) {
    let mut all_files = std::collections::HashSet::new();
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
        if !is_focus_run || test.focused {
            all_files.insert(test.file);
            tests.push(*test);
        }
    });

    // Sort alphabetically for deterministic run order
    tests.sort_by_key(|test| test.file);

    (tests, all_files.len(), is_focus_run)
}

pub struct TestContext {
    scene_tree: Gd<Node>,
}

#[derive(Copy, Clone)]
struct RustTestCase {
    name: &'static str,
    file: &'static str,
    skipped: bool,
    /// If one or more tests are focused, only they will be executed. Helpful for debugging and working on specific features.
    focused: bool,
    #[allow(dead_code)]
    line: u32,
    function: fn(&TestContext),
}
