/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::bind::{godot_api, GodotClass};
use godot::init::{gdextension, ExtensionLibrary};
use godot::sys;
use godot::test::itest;
use std::collections::HashSet;
use std::panic;

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

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Entry point + main runner class

#[gdextension(entry_point=itest_init)]
unsafe impl ExtensionLibrary for IntegrationTests {}

#[derive(GodotClass, Debug)]
#[class(base=Node, init)]
struct IntegrationTests {
    tests_run: i64,
    tests_passed: i64,
    tests_skipped: i64,
}

#[godot_api]
impl IntegrationTests {
    // TODO could return a Stats object with properties in the future
    #[func]
    fn test_all(&mut self) {
        println!("Run Godot integration tests...");
        self.run_tests();
    }

    #[func]
    fn num_run(&self) -> i64 {
        self.tests_run
    }

    #[func]
    fn num_passed(&self) -> i64 {
        self.tests_passed
    }

    #[func]
    fn num_skipped(&self) -> i64 {
        self.tests_skipped
    }

    fn run_tests(&mut self) {
        let mut tests: Vec<TestCase> = vec![];

        let mut all_files = HashSet::new();
        sys::plugin_foreach!(__GODOT_ITEST; |test: &TestCase| {
            all_files.insert(test.file);
            tests.push(*test);
        });

        println!(
            "Rust: found {} tests in {} files.",
            tests.len(),
            all_files.len()
        );

        let mut last_file = None;
        for test in tests {
            let outcome = run_test(&test);

            self.tests_run += 1;
            match outcome {
                TestOutcome::Passed => self.tests_passed += 1,
                TestOutcome::Failed => {}
                TestOutcome::Skipped => self.tests_skipped += 1,
            }

            print_test(&test, outcome, &mut last_file);
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Implementation

// Registers all the tests
sys::plugin_registry!(__GODOT_ITEST: TestCase);

// For more colors, see https://stackoverflow.com/a/54062826
// To experiment with colors, add `rand` dependency and add following code above.
//     use rand::seq::SliceRandom;
//     let outcome = [TestOutcome::Passed, TestOutcome::Failed, TestOutcome::Skipped];
//     let outcome = outcome.choose(&mut rand::thread_rng()).unwrap();
const FMT_GREEN: &str = "\x1b[32m";
const FMT_YELLOW: &str = "\x1b[33m";
const FMT_RED: &str = "\x1b[31m";
const FMT_END: &str = "\x1b[0m";

fn run_test(test: &TestCase) -> TestOutcome {
    if test.skipped {
        return TestOutcome::Skipped;
    }

    // Explicit type to prevent tests from returning a value
    let success: Option<()> =
        godot::private::handle_panic(|| format!("   !! Test {} failed", test.name), test.function);

    if success.is_some() {
        TestOutcome::Passed
    } else {
        TestOutcome::Failed
    }
}

/// Prints a test name and its outcome.
///
/// Note that this is run after a test run, so stdout/stderr output during the test will be printed before.
fn print_test(test: &TestCase, outcome: TestOutcome, last_file: &mut Option<&'static str>) {
    // Check if we need to open a new category for a file
    let print_file = last_file.map_or(true, |last_file| last_file != test.file);
    if print_file {
        let sep_pos = test.file.rfind(&['/', '\\']).unwrap_or(0);
        println!("\n   {}:", &test.file[sep_pos + 1..]);
    }

    // Do not use print_rich() from Godot, because it's very slow and significantly delays test execution.
    let test_name = test.name;
    let end = FMT_END;
    let (col, outcome) = match outcome {
        TestOutcome::Passed => (FMT_GREEN, "ok"),
        TestOutcome::Failed => (FMT_RED, "FAILED"),
        TestOutcome::Skipped => (FMT_YELLOW, "ignored"),
    };

    println!("   -- {test_name} ... {col}{outcome}{end}");

    // State update for file-category-print
    *last_file = Some(test.file);
}

pub(crate) fn expect_panic(context: &str, code: impl FnOnce() + panic::UnwindSafe) {
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

#[derive(Copy, Clone)]
struct TestCase {
    name: &'static str,
    file: &'static str,
    skipped: bool,
    #[allow(dead_code)]
    line: u32,
    function: fn(),
}

#[must_use]
enum TestOutcome {
    Passed,
    Failed,
    Skipped,
}
