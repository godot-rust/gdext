/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::bind::{godot_api, GodotClass};
use godot::init::{gdextension, ExtensionLibrary};
use godot::sys;
use godot::test::itest;
use std::panic::UnwindSafe;

mod array_test;
mod base_test;
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


#[derive(Copy, Clone)]
pub struct TestCase {
    name: &'static str,
    function: fn(),
}

#[must_use]
fn run_test(test: &TestCase) -> bool {
    println!("   -- {}", test.name);

    // Explicit type to prevent tests from returning a value
    let success: Option<()> =
        godot::private::handle_panic(|| format!("   !! Test {} failed", test.name), test.function);

    success.is_some()
}

sys::plugin_registry!(__GODOT_ITEST: TestCase);

// fn register_classes() {
//     object_test::register();
//     gdscript_ffi_test::register();
//     virtual_methods_test::register();
// }

fn run_tests() -> bool {
    let mut tests: Vec<TestCase> = vec![];

    sys::plugin_foreach!(__GODOT_ITEST; |test: &TestCase| {
        tests.push(*test);
    });

    println!("Collected {} tests.", tests.len());


    let mut stats = TestStats::default();
    for test in tests {
        stats.tests_run += 1;
        if run_test(&test) {
            stats.tests_passed += 1;
        }
    }

    stats.tests_run == stats.tests_passed
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Implementation

#[gdextension(entry_point=itest_init)]
unsafe impl ExtensionLibrary for IntegrationTests {}

#[derive(GodotClass, Debug)]
#[class(base=Node, init)]
struct IntegrationTests {}

#[godot_api]
impl IntegrationTests {
    #[func]
    fn test_all(&mut self) -> bool {
        println!("Run Godot integration tests...");
        run_tests()
    }
}

pub(crate) fn expect_panic(context: &str, code: impl FnOnce() + UnwindSafe) {
    let panic = std::panic::catch_unwind(code);
    assert!(
        panic.is_err(),
        "code should have panicked but did not: {context}",
    );
}

#[derive(Default)]
struct TestStats {
    tests_run: usize,
    tests_passed: usize,
    tests_skipped: usize,
}
