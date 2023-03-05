/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::bind::{godot_api, GodotClass};
use godot::builtin::{Array, ToVariant, Variant};

use crate::RustTestCase;
use std::time::{Duration, Instant};

#[derive(GodotClass, Debug)]
#[class(init)]
pub(crate) struct IntegrationTests {
    total: i64,
    passed: i64,
    skipped: i64,
}

#[godot_api]
impl IntegrationTests {
    #[allow(clippy::uninlined_format_args)]
    #[func]
    fn run_all_tests(&mut self, gdscript_tests: Array, gdscript_file_count: i64) -> bool {
        println!(
            "{}Run{} Godot integration tests...",
            FMT_GREEN_BOLD, FMT_END
        );

        let (rust_tests, rust_file_count) = super::collect_rust_tests();
        println!(
            "  Rust: found {} tests in {} files.",
            rust_tests.len(),
            rust_file_count
        );
        println!(
            "  GDScript: found {} tests in {} files.",
            gdscript_tests.len(),
            gdscript_file_count
        );

        let clock = Instant::now();
        self.run_rust_tests(rust_tests);
        let rust_time = clock.elapsed();
        self.run_gdscript_tests(gdscript_tests);
        let gdscript_time = clock.elapsed() - rust_time;

        self.conclude(rust_time, gdscript_time)
    }

    fn run_rust_tests(&mut self, tests: Vec<RustTestCase>) {
        let mut last_file = None;
        for test in tests {
            let outcome = run_rust_test(&test);

            self.update_stats(&outcome);
            print_test(test.file.to_string(), test.name, outcome, &mut last_file);
        }
    }

    fn run_gdscript_tests(&mut self, tests: Array) {
        let mut last_file = None;
        for test in tests.iter_shared() {
            let result = test.call("run", &[]);
            let success = result.try_to::<bool>().unwrap_or_else(|_| {
                panic!("GDScript test case {test} returned non-bool: {result}")
            });

            let test_file = get_property(&test, "suite_name");
            let test_case = get_property(&test, "method_name");
            let outcome = TestOutcome::from_bool(success);

            self.update_stats(&outcome);
            print_test(test_file, &test_case, outcome, &mut last_file);
        }
    }

    fn conclude(&self, rust_time: Duration, gdscript_time: Duration) -> bool {
        let Self {
            total,
            passed,
            skipped,
            ..
        } = *self;

        // Consider 0 tests run as a failure too, because it's probably a problem with the run itself.
        let failed = total - passed - skipped;
        let all_passed = failed == 0 && total != 0;

        let outcome = TestOutcome::Passed; // TODO

        let rust_time = rust_time.as_secs_f32();
        let gdscript_time = gdscript_time.as_secs_f32();
        let total_time = rust_time + gdscript_time;

        println!("\nTest result: {outcome}. {passed} passed; {failed} failed.");
        println!("  Time: {total_time:.2}s.  (Rust {rust_time:.2}s, GDScript {gdscript_time:.2}s)");
        all_passed
    }

    fn update_stats(&mut self, outcome: &TestOutcome) {
        self.total += 1;
        match outcome {
            TestOutcome::Passed => self.passed += 1,
            TestOutcome::Failed => {}
            TestOutcome::Skipped => self.skipped += 1,
        }
    }
}

// For more colors, see https://stackoverflow.com/a/54062826
// To experiment with colors, add `rand` dependency and add following code above.
//     use rand::seq::SliceRandom;
//     let outcome = [TestOutcome::Passed, TestOutcome::Failed, TestOutcome::Skipped];
//     let outcome = outcome.choose(&mut rand::thread_rng()).unwrap();
const FMT_GREEN_BOLD: &str = "\x1b[32;1;1m";
const FMT_GREEN: &str = "\x1b[32m";
const FMT_YELLOW: &str = "\x1b[33m";
const FMT_RED: &str = "\x1b[31m";
const FMT_END: &str = "\x1b[0m";

fn run_rust_test(test: &RustTestCase) -> TestOutcome {
    if test.skipped {
        return TestOutcome::Skipped;
    }

    // Explicit type to prevent tests from returning a value
    let success: Option<()> =
        godot::private::handle_panic(|| format!("   !! Test {} failed", test.name), test.function);

    TestOutcome::from_bool(success.is_some())
}

/// Prints a test name and its outcome.
///
/// Note that this is run after a test run, so stdout/stderr output during the test will be printed before.
/// It would be possible to print the test name before and the outcome after, but that would split or duplicate the line.
fn print_test(
    test_file: String,
    test_case: &str,
    outcome: TestOutcome,
    last_file: &mut Option<String>,
) {
    // Check if we need to open a new category for a file
    let print_file = last_file
        .as_ref()
        .map_or(true, |last_file| last_file != &test_file);

    if print_file {
        let file_subtitle = if let Some(sep_pos) = test_file.rfind(&['/', '\\']) {
            &test_file[sep_pos + 1..]
        } else {
            test_file.as_str()
        };

        println!("\n   {file_subtitle}:");
    }

    println!("   -- {test_case} ... {outcome}");

    // State update for file-category-print
    *last_file = Some(test_file);
}

fn get_property(test: &Variant, property: &str) -> String {
    test.call("get", &[property.to_variant()]).to::<String>()
}

#[must_use]
enum TestOutcome {
    Passed,
    Failed,
    Skipped,
}

impl TestOutcome {
    fn from_bool(success: bool) -> Self {
        if success {
            Self::Passed
        } else {
            Self::Failed
        }
    }
}

impl std::fmt::Display for TestOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Do not use print_rich() from Godot, because it's very slow and significantly delays test execution.
        let end = FMT_END;
        let (col, outcome) = match self {
            TestOutcome::Passed => (FMT_GREEN, "ok"),
            TestOutcome::Failed => (FMT_RED, "FAILED"),
            TestOutcome::Skipped => (FMT_YELLOW, "ignored"),
        };

        write!(f, "{col}{outcome}{end}")
    }
}
