/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::time::{Duration, Instant};

use godot::builtin::{vslice, Array, Callable, GString, Variant, VariantArray};
use godot::classes::{Engine, Node, Os};
use godot::global::godot_error;
use godot::obj::Gd;
use godot::register::{godot_api, GodotClass};

use super::AsyncRustTestCase;
use crate::framework::{
    bencher, passes_filter, BenchResult, RustBenchmark, RustTestCase, TestContext,
};

#[derive(Debug, Clone, Default)]
struct TestStats {
    total: usize,
    passed: usize,
    skipped: usize,
    failed_list: Vec<String>,
}

#[derive(GodotClass, Debug)]
#[class(init)]
pub struct IntegrationTests {
    stats: TestStats,
    focus_run: bool,
}

#[godot_api]
impl IntegrationTests {
    #[allow(clippy::uninlined_format_args)]
    #[allow(clippy::too_many_arguments)]
    #[func]
    fn run_all_tests(
        &mut self,
        gdscript_tests: VariantArray,
        gdscript_file_count: i64,
        allow_focus: bool,
        scene_tree: Gd<Node>,
        filters: VariantArray,
        property_tests: Gd<Node>,
        on_finished: Callable,
    ) {
        println!("{}Run{} Godot integration tests...", FMT_CYAN_BOLD, FMT_END);
        let filters: Vec<String> = filters.iter_shared().map(|v| v.to::<String>()).collect();
        let gdscript_tests = gdscript_tests
            .iter_shared()
            .filter(|test| {
                let test_name = get_property(test, "method_name");
                passes_filter(filters.as_slice(), &test_name)
            })
            .collect::<Array<_>>();

        let rust_test_cases = collect_rust_tests(&filters);

        // Print based on focus/not focus.
        self.focus_run = rust_test_cases.focus_run;
        if rust_test_cases.focus_run {
            println!("  {FMT_CYAN}Focused run{FMT_END} -- execute only selected Rust tests.")
        }
        println!(
            "  Rust: found {} tests in {} files.",
            rust_test_cases.rust_test_count, rust_test_cases.rust_file_count
        );
        if !rust_test_cases.focus_run {
            println!(
                "  GDScript: found {} tests in {} files.",
                gdscript_tests.len(),
                gdscript_file_count
            );
        }

        let clock = Instant::now();
        self.run_rust_tests(
            rust_test_cases.rust_tests,
            scene_tree.clone(),
            property_tests.clone(),
        );
        let rust_time = clock.elapsed();

        let gdscript_time = if !rust_test_cases.focus_run {
            let extra_duration = self.run_gdscript_tests(gdscript_tests);
            Some((clock.elapsed() - rust_time, extra_duration))
        } else {
            None
        };

        {
            let stats = self.stats.clone();

            let on_finalize_test = move |stats, property_tests: Gd<Node>| {
                let gdscript_elapsed = gdscript_time
                    .as_ref()
                    .map(|gdtime| gdtime.0)
                    .unwrap_or_default();

                let rust_async_time = clock.elapsed() - rust_time - gdscript_elapsed;

                property_tests.free();

                let result = Self::conclude_tests(
                    &stats,
                    rust_time + rust_async_time,
                    gdscript_time.map(|(elapsed, extra)| elapsed + extra),
                    allow_focus,
                );

                // Calling deferred to break a potentially synchronous call stack and avoid re-entrancy.
                on_finished.call_deferred(vslice![result]);
            };

            Self::run_async_rust_tests(
                stats,
                rust_test_cases.async_rust_tests,
                scene_tree,
                property_tests,
                on_finalize_test,
            );
        }
    }

    /// Queried by some `.gd` tests to check whether they can call into conditionally-compiled Rust classes.
    #[func]
    fn is_full_codegen() -> bool {
        cfg!(feature = "codegen-full")
    }

    #[allow(clippy::uninlined_format_args)]
    #[func]
    fn run_all_benchmarks(&mut self, scene_tree: Gd<Node>) {
        if self.focus_run {
            println!("  Benchmarks skipped (focused run).");
            return;
        }

        println!("\n\n{}Run{} Godot benchmarks...", FMT_CYAN_BOLD, FMT_END);

        self.warn_if_debug();

        let (benchmarks, rust_file_count) = super::collect_rust_benchmarks();
        println!(
            "  Rust: found {} benchmarks in {} files.",
            benchmarks.len(),
            rust_file_count
        );

        let clock = Instant::now();
        self.run_rust_benchmarks(benchmarks, scene_tree);
        let total_time = clock.elapsed();
        self.conclude_benchmarks(total_time);
    }

    fn warn_if_debug(&self) {
        let rust_debug = cfg!(debug_assertions);
        let godot_debug = Os::singleton().is_debug_build();

        let what = match (rust_debug, godot_debug) {
            (true, true) => Some("both Rust and Godot engine use debug builds"),
            (true, false) => Some("Rust uses a debug build"),
            (false, true) => Some("Godot engine uses a debug build"),
            (false, false) => None,
        };

        if let Some(what) = what {
            println!("{FMT_YELLOW}  Warning: {what}, benchmarks may not be expressive.{FMT_END}");
        }
    }

    fn run_rust_tests(
        &mut self,
        tests: Vec<RustTestCase>,
        scene_tree: Gd<Node>,
        property_tests: Gd<Node>,
    ) {
        let ctx = TestContext {
            scene_tree,
            property_tests,
        };

        let mut last_file = None;
        for test in tests {
            print_test_pre(test.name, test.file, last_file.as_deref(), false);
            last_file = Some(test.file.to_string());

            let outcome = run_rust_test(&test, &ctx);

            Self::update_stats(&mut self.stats, &outcome, test.file, test.name);
            print_test_post(test.name, outcome);
        }
    }

    fn run_async_rust_tests(
        stats: TestStats,
        tests: Vec<AsyncRustTestCase>,
        scene_tree: Gd<Node>,
        property_tests: Gd<Node>,
        on_finalize_test: impl FnOnce(TestStats, Gd<Node>) + 'static,
    ) {
        let mut tests_iter = tests.into_iter();

        let Some(first_test) = tests_iter.next() else {
            return on_finalize_test(stats, property_tests);
        };

        let ctx = TestContext {
            scene_tree,
            property_tests,
        };

        Self::run_async_rust_tests_step(tests_iter, first_test, ctx, stats, None, on_finalize_test);
    }

    fn run_async_rust_tests_step(
        mut tests_iter: impl Iterator<Item = AsyncRustTestCase> + 'static,
        test: AsyncRustTestCase,
        ctx: TestContext,
        mut stats: TestStats,
        mut last_file: Option<String>,
        on_finalize_test: impl FnOnce(TestStats, Gd<Node>) + 'static,
    ) {
        print_test_pre(test.name, test.file, last_file.as_deref(), true);
        last_file.replace(test.file.to_string());

        run_async_rust_test(&test, &ctx.clone(), move |outcome| {
            Self::update_stats(&mut stats, &outcome, test.file, test.name);
            print_test_post(test.name, outcome);

            if let Some(next) = tests_iter.next() {
                return Self::run_async_rust_tests_step(
                    tests_iter,
                    next,
                    ctx,
                    stats,
                    last_file,
                    on_finalize_test,
                );
            }

            on_finalize_test(stats, ctx.property_tests);
        });
    }

    fn run_gdscript_tests(&mut self, tests: VariantArray) -> Duration {
        let mut last_file = None;
        let mut extra_duration = Duration::new(0, 0);

        for test in tests.iter_shared() {
            let test_file = get_property(&test, "suite_name");
            let test_case = get_property(&test, "method_name");

            print_test_pre(&test_case, &test_file, last_file.as_deref(), true);

            last_file = Some(test_file.clone());

            // If GDScript invokes Rust code that fails, the panic would break through; catch it.
            // TODO(bromeon): use try_call() once available.
            let result = std::panic::catch_unwind(|| test.call("run", &[]));

            // In case a test needs to disable error messages, to ensure it runs properly.
            Engine::singleton().set_print_error_messages(true);

            if let Some(duration) = get_execution_time(&test) {
                extra_duration += duration;
            }

            let outcome = match result {
                Ok(result) => {
                    let success = result.try_to::<bool>().unwrap_or_else(|_| {
                        // Not a failing test, but an error in the test setup.
                        panic!("GDScript test case {test} returned non-bool: {result}")
                    });

                    for error in get_errors(&test).iter_shared() {
                        godot_error!("{error}");
                    }
                    TestOutcome::from_bool(success)
                }
                Err(e) => {
                    // TODO(bromeon) should this be a fatal error, i.e. panicking and aborting tests -> bad test setup?
                    // If GDScript receives panics, this can also happen in user code that is _not_ invoked from Rust, and thus a panic
                    // could not be caught, causing UB at the Godot FFI boundary (in practice, this will be a defined Godot crash with
                    // stack trace though).
                    godot_error!("GDScript test panicked");
                    godot::private::extract_panic_message(&e);
                    TestOutcome::Failed
                }
            };

            Self::update_stats(&mut self.stats, &outcome, &test_file, &test_case);
            print_test_post(&test_case, outcome);
        }
        extra_duration
    }

    fn conclude_tests(
        stats: &TestStats,
        rust_time: Duration,
        gdscript_time: Option<Duration>,
        allow_focus: bool,
    ) -> bool {
        let TestStats {
            total,
            passed,
            skipped,
            ..
        } = stats;

        // Consider 0 tests run as a failure too, because it's probably a problem with the run itself.
        let failed = total - passed - skipped;
        let all_passed = failed == 0 && *total != 0;

        let outcome = TestOutcome::from_bool(all_passed);

        let rust_time = rust_time.as_secs_f32();
        let gdscript_time = gdscript_time.map(|t| t.as_secs_f32());
        let focused_run = gdscript_time.is_none();

        let extra = if *skipped > 0 {
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

        if !all_passed {
            println!("\n  Failed tests:");
            let max = 10;
            for test in stats.failed_list.iter().take(max) {
                println!("  * {test}");
            }

            if stats.failed_list.len() > max {
                println!("  * ... and {} more.", stats.failed_list.len() - max);
            }

            println!();
        }

        if focused_run && !allow_focus {
            println!("  {FMT_YELLOW}Focus run disallowed; return failure.{FMT_END}");
            false
        } else {
            all_passed
        }
    }

    fn run_rust_benchmarks(&mut self, benchmarks: Vec<RustBenchmark>, _scene_tree: Gd<Node>) {
        // let ctx = TestContext { scene_tree };

        print!("\n{FMT_CYAN}{space}", space = " ".repeat(36));
        for metrics in bencher::metrics() {
            print!("{metrics:>13}");
        }
        print!("{FMT_END}");

        let mut last_file = None;
        for bench in benchmarks {
            print_bench_pre(bench.name, bench.file, last_file.as_deref());
            last_file = Some(bench.file.to_string());

            let result = bencher::run_benchmark(bench.function, bench.repetitions);
            print_bench_post(result);
        }
    }

    fn conclude_benchmarks(&self, total_time: Duration) {
        let secs = total_time.as_secs_f32();
        println!("\nBenchmarks completed in {secs:.2}s.");
    }

    fn update_stats(
        stats: &mut TestStats,
        outcome: &TestOutcome,
        test_file: &str,
        test_name: &str,
    ) {
        stats.total += 1;
        match outcome {
            TestOutcome::Passed => stats.passed += 1,
            TestOutcome::Failed => stats.failed_list.push(format!(
                "{} > {}",
                extract_file_subtitle(test_file),
                test_name
            )),
            TestOutcome::Skipped => stats.skipped += 1,
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

    // This will appear in all panics, but those inside expect_panic() are suppressed.
    // So the "itest failed" message will only appear for unexpected panics, where tests indeed fail.
    let err_context = || format!("itest `{}` failed", test.name);

    // Explicit type to prevent tests from returning a value.
    let success: Result<(), _> = godot::private::handle_panic(err_context, || (test.function)(ctx));

    TestOutcome::from_bool(success.is_ok())
}

fn run_async_rust_test(
    test: &AsyncRustTestCase,
    ctx: &TestContext,
    on_test_finished: impl FnOnce(TestOutcome) + 'static,
) {
    if test.skipped {
        return on_test_finished(TestOutcome::Skipped);
    }

    // Explicit type to prevent tests from returning a value
    let err_context = || format!("itest `{}` failed", test.name);
    let success: Result<godot::task::TaskHandle, _> =
        godot::private::handle_panic(err_context, || (test.function)(ctx));

    let Ok(task_handle) = success else {
        return on_test_finished(TestOutcome::Failed);
    };

    check_async_test_task(task_handle, on_test_finished, ctx);
}

fn check_async_test_task(
    task_handle: godot::task::TaskHandle,
    on_test_finished: impl FnOnce(TestOutcome) + 'static,
    ctx: &TestContext,
) {
    use godot::classes::object::ConnectFlags;
    use godot::obj::EngineBitfield;
    use godot::task::has_godot_task_panicked;

    if !task_handle.is_pending() {
        on_test_finished(TestOutcome::from_bool(!has_godot_task_panicked(
            task_handle,
        )));

        return;
    }

    let next_ctx = ctx.clone();
    let mut callback = Some(on_test_finished);
    let mut probably_task_handle = Some(task_handle);

    let deferred = Callable::from_local_fn("run_async_rust_test", move |_| {
        check_async_test_task(
            probably_task_handle
                .take()
                .expect("Callable will only be called once!"),
            callback
                .take()
                .expect("Callable should not be called multiple times!"),
            &next_ctx,
        );
        Ok(Variant::nil())
    });

    ctx.scene_tree
        .get_tree()
        .expect("The itest scene tree node is part of a Godot SceneTree")
        .connect_ex("process_frame", &deferred)
        .flags(ConnectFlags::ONE_SHOT.ord() as u32)
        .done();
}

fn print_test_pre(test_case: &str, test_file: &str, last_file: Option<&str>, flush: bool) {
    print_file_header(test_file, last_file);

    print!("   -- {test_case} ... ");
    if flush {
        // Flush in GDScript, because its own print may come sooner than Rust prints otherwise.
        // (Strictly speaking, this can also happen from Rust, when Godot prints something. So far, it didn't though...)
        use std::io::Write;
        std::io::stdout().flush().expect("flush stdout");
    }
}

fn print_file_header(file: &str, last_file: Option<&str>) {
    // Check if we need to open a new category for a file.
    let print_file = last_file != Some(file);

    if print_file {
        println!("\n   {}:", extract_file_subtitle(file));
    }
}

fn extract_file_subtitle(file: &str) -> &str {
    if let Some(sep_pos) = file.rfind(['/', '\\']) {
        &file[sep_pos + 1..]
    } else {
        file
    }
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

fn print_bench_pre(benchmark: &str, bench_file: &str, last_file: Option<&str>) {
    print_file_header(bench_file, last_file);

    let benchmark = if benchmark.len() > 26 {
        &benchmark[..26]
    } else {
        benchmark
    };

    print!("   -- {benchmark:<26} ...");
}

fn print_bench_post(result: BenchResult) {
    for stat in result.stats.iter() {
        print!(" {:>10.3}μs", stat.as_nanos() as f64 / 1000.0);
    }
    println!();
}

fn get_property(test: &Variant, property: &str) -> String {
    test.call("get", vslice![property]).to::<String>()
}

fn get_execution_time(test: &Variant) -> Option<Duration> {
    let seconds = test
        .call("get", vslice!["execution_time_seconds"])
        .try_to::<f64>()
        .ok()?;

    Some(Duration::from_secs_f64(seconds))
}

fn get_errors(test: &Variant) -> Array<GString> {
    test.call("get", vslice!["errors"])
        .try_to::<Array<GString>>()
        .unwrap_or_default()
}

struct RustTestCases {
    rust_tests: Vec<RustTestCase>,
    async_rust_tests: Vec<AsyncRustTestCase>,
    rust_test_count: usize,
    rust_file_count: usize,
    focus_run: bool,
}

fn collect_rust_tests(filters: &[String]) -> RustTestCases {
    let (mut rust_tests, mut rust_files, focus_run) = super::collect_rust_tests(filters);

    let (async_rust_tests, async_rust_files, async_focus_run) =
        super::collect_async_rust_tests(filters, focus_run);

    if !focus_run && async_focus_run {
        rust_tests.clear();
        rust_files.clear();
    }

    let rust_test_count = rust_tests.len() + async_rust_tests.len();
    let rust_file_count = rust_files.union(&async_rust_files).count();

    RustTestCases {
        rust_tests,
        async_rust_tests,
        rust_test_count,
        rust_file_count,
        focus_run: focus_run || async_focus_run,
    }
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
