/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::time::{Duration, Instant};

use godot::bind::{godot_api, GodotClass};
use godot::builtin::{Array, GodotString, ToVariant, Variant, VariantArray};
use godot::engine::Node;
use godot::log::godot_error;
use godot::obj::Gd;

use crate::{RustTestCase, TestContext};

#[derive(GodotClass, Debug)]
#[class(init)]
pub(crate) struct IntegrationTests {
    total: i64,
    passed: i64,
    skipped: i64,
    focus_run: bool,
}

#[godot_api]
impl IntegrationTests {
    #[allow(clippy::uninlined_format_args)]
    #[func]
    fn run_all_tests(
        &mut self,
        gdscript_tests: VariantArray,
        gdscript_file_count: i64,
        allow_focus: bool,
        scene_tree: Gd<Node>,
    ) -> bool {
        println!("{}Run{} Godot integration tests...", FMT_CYAN_BOLD, FMT_END);

        let (rust_tests, rust_file_count, focus_run) = super::collect_rust_tests();
        self.focus_run = focus_run;
        if focus_run {
            println!("  {FMT_CYAN}Focused run{FMT_END} -- execute only selected Rust tests.")
        }
        println!(
            "  Rust: found {} tests in {} files.",
            rust_tests.len(),
            rust_file_count
        );
        if !focus_run {
            println!(
                "  GDScript: found {} tests in {} files.",
                gdscript_tests.len(),
                gdscript_file_count
            );
        }

        let clock = Instant::now();
        self.run_rust_tests(rust_tests, scene_tree);
        let rust_time = clock.elapsed();
        let gdscript_time = if !focus_run {
            let extra_duration = self.run_gdscript_tests(gdscript_tests);
            Some((clock.elapsed() - rust_time) + extra_duration)
        } else {
            None
        };

        self.conclude(rust_time, gdscript_time, allow_focus)
    }

    fn run_rust_tests(&mut self, tests: Vec<RustTestCase>, scene_tree: Gd<Node>) {
        let ctx = TestContext { scene_tree };

        let mut last_file = None;
        for test in tests {
            print_test_pre(test.name, test.file.to_string(), &mut last_file, false);
            let outcome = run_rust_test(&test, &ctx);

            self.update_stats(&outcome);
            print_test_post(test.name, outcome);
        }
    }

    fn run_gdscript_tests(&mut self, tests: VariantArray) -> Duration {
        let mut last_file = None;
        let mut extra_duration = Duration::new(0, 0);

        for test in tests.iter_shared() {
            let test_file = get_property(&test, "suite_name");
            let test_case = get_property(&test, "method_name");

            print_test_pre(&test_case, test_file, &mut last_file, true);
            let result = test.call("run", &[]);
            if let Some(duration) = get_execution_time(&test) {
                extra_duration += duration;
            }
            let success = result.try_to::<bool>().unwrap_or_else(|_| {
                panic!("GDScript test case {test} returned non-bool: {result}")
            });
            for error in get_errors(&test).iter_shared() {
                godot_error!("{error}");
            }
            let outcome = TestOutcome::from_bool(success);

            self.update_stats(&outcome);
            print_test_post(&test_case, outcome);
        }
        extra_duration
    }

    fn conclude(
        &self,
        rust_time: Duration,
        gdscript_time: Option<Duration>,
        allow_focus: bool,
    ) -> bool {
        let Self {
            total,
            passed,
            skipped,
            ..
        } = *self;

        // Consider 0 tests run as a failure too, because it's probably a problem with the run itself.
        let failed = total - passed - skipped;
        let all_passed = failed == 0 && total != 0;

        let outcome = TestOutcome::from_bool(all_passed);

        let rust_time = rust_time.as_secs_f32();
        let gdscript_time = gdscript_time.map(|t| t.as_secs_f32());
        let focused_run = gdscript_time.is_none();

        let extra = if skipped > 0 {
            format!(", {skipped} skipped")
        } else if focused_run {
            " (focused run)".to_string()
        } else {
            "".to_string()
        };

        println!("\nTest result: {outcome}. {passed} passed; {failed} failed{extra}.");
        if let Some(gdscript_time) = gdscript_time {
            let total_time = rust_time + gdscript_time;
            println!(
                "  Time: {total_time:.2}s.  (Rust {rust_time:.2}s, GDScript {gdscript_time:.2}s)"
            );
        } else {
            println!("  Time: {rust_time:.2}s.");
        }

        if focused_run && !allow_focus {
            println!("  {FMT_YELLOW}Focus run disallowed; return failure.{FMT_END}");
            false
        } else {
            all_passed
        }
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
const FMT_CYAN_BOLD: &str = "\x1b[36;1;1m";
const FMT_CYAN: &str = "\x1b[36m";
const FMT_GREEN: &str = "\x1b[32m";
const FMT_YELLOW: &str = "\x1b[33m";
const FMT_RED: &str = "\x1b[31m";
const FMT_END: &str = "\x1b[0m";

fn run_rust_test(test: &RustTestCase, ctx: &TestContext) -> TestOutcome {
    if test.skipped {
        return TestOutcome::Skipped;
    }

    // Explicit type to prevent tests from returning a value
    let err_context = || format!("itest `{}` failed", test.name);
    let success: Option<()> = godot::private::handle_panic(err_context, || (test.function)(ctx));

    TestOutcome::from_bool(success.is_some())
}

fn print_test_pre(test_case: &str, test_file: String, last_file: &mut Option<String>, flush: bool) {
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

    print!("   -- {test_case} ... ");
    if flush {
        // Flush in GDScript, because its own print may come sooner than Rust prints otherwise
        // (strictly speaking, this can also happen from Rust, when Godot prints something. So far, it didn't though...
        godot::private::flush_stdout();
    }

    // State update for file-category-print
    *last_file = Some(test_file);
}

/// Prints a test name and its outcome.
///
/// Note that this is run after a test run, so stdout/stderr output during the test will be printed before.
/// It would be possible to print the test name before and the outcome after, but that would split or duplicate the line.
fn print_test_post(test_case: &str, outcome: TestOutcome) {
    // If test failed, something was printed (e.g. assertion), so we can print the entire line again; otherwise just outcome on same line.
    if matches!(outcome, TestOutcome::Failed) {
        println!("   -- {test_case} ... {outcome}");
    } else {
        println!("{outcome}");
    }
}

fn get_property(test: &Variant, property: &str) -> String {
    test.call("get", &[property.to_variant()]).to::<String>()
}

fn get_execution_time(test: &Variant) -> Option<Duration> {
    let seconds = test
        .call("get", &["execution_time_seconds".to_variant()])
        .try_to::<f64>()
        .ok()?;
    Some(Duration::from_secs_f64(seconds))
}

fn get_errors(test: &Variant) -> Array<GodotString> {
    test.call("get", &["errors".to_variant()])
        .try_to::<Array<GodotString>>()
        .unwrap_or(Array::new())
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
            TestOutcome::Skipped => (FMT_YELLOW, "skipped"),
        };

        write!(f, "{col}{outcome}{end}")
    }
}
