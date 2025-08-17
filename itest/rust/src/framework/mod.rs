/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::collections::HashSet;
use std::panic;

use godot::classes::{Engine, Node, Os, SceneTree};
use godot::obj::Gd;
use godot::sys;

mod bencher;
mod runner;

pub use bencher::*;
/// Allow re-import as `crate::framework::itest`.
pub use godot::test::{bench, itest};
pub use runner::*;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Plugin registration

// Registers all the `#[itest]` tests and `#[bench]` benchmarks.
sys::plugin_registry!(pub(crate) __GODOT_ITEST: RustTestCase);
#[cfg(since_api = "4.2")]
sys::plugin_registry!(pub(crate) __GODOT_ASYNC_ITEST: AsyncRustTestCase);
sys::plugin_registry!(pub(crate) __GODOT_BENCH: RustBenchmark);

/// Finds all `#[itest]` tests.
fn collect_rust_tests(filters: &[String]) -> (Vec<RustTestCase>, HashSet<&str>, bool) {
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

    (tests, all_files, is_focus_run)
}

/// Finds all `#[itest(async)]` tests.
#[cfg(since_api = "4.2")]
fn collect_async_rust_tests(
    filters: &[String],
    sync_focus_run: bool,
) -> (Vec<AsyncRustTestCase>, HashSet<&str>, bool) {
    let mut all_files = HashSet::new();
    let mut tests = vec![];
    let mut is_focus_run = sync_focus_run;

    sys::plugin_foreach!(__GODOT_ASYNC_ITEST; |test: &AsyncRustTestCase| {
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

    (tests, all_files, is_focus_run)
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
#[derive(Clone)]
pub struct TestContext {
    pub scene_tree: Gd<Node>,
    pub property_tests: Gd<Node>,
}

/// Utility to assert that something can be sent between threads.
pub struct ThreadCrosser<T> {
    value: T,
}

impl<T> ThreadCrosser<T> {
    pub fn new(value: T) -> Self {
        Self { value }
    }

    /// # Safety
    /// Bypasses `Send` checks, user's responsibility.
    pub unsafe fn extract(self) -> T {
        self.value
    }
}

unsafe impl<T> Send for ThreadCrosser<T> {}

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

#[cfg(since_api = "4.2")]
#[derive(Copy, Clone)]
pub struct AsyncRustTestCase {
    pub name: &'static str,
    pub file: &'static str,
    pub skipped: bool,
    /// If one or more tests are focused, only they will be executed. Helpful for debugging and working on specific features.
    pub focused: bool,
    #[allow(dead_code)]
    pub line: u32,
    pub function: fn(&TestContext) -> godot::task::TaskHandle,
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

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Toolbox for tests

/// Swaps panic hooks, to disable printing during expected panics. Also disables gdext's panic printing.
pub fn suppress_panic_log<R>(callback: impl FnOnce() -> R) -> R {
    // DISABLE following lines to *temporarily* debug panics.
    // Note that they currently print "itest `{}` failed", even if the test doesn't fail (which isn't usually relevant in suppressed mode).
    let prev_hook = panic::take_hook();
    panic::set_hook(Box::new(
        |_panic_info| { /* suppress panic hook; do nothing */ },
    ));

    // Keep following lines.
    let prev_print_level = godot::private::set_error_print_level(0);
    let res = callback();
    godot::private::set_error_print_level(prev_print_level);

    // DISABLE following line to *temporarily* debug panics.
    panic::set_hook(prev_hook);

    res
}

pub fn expect_panic(context: &str, code: impl FnOnce()) {
    // Generally, types should be unwind safe, and this helps ergonomics in testing (especially around &mut in expect_panic closures).
    let code = panic::AssertUnwindSafe(code);
    let panic = suppress_panic_log(move || panic::catch_unwind(code));

    assert!(
        panic.is_err(),
        "code should have panicked but did not: {context}",
    );
}

pin_project_lite::pin_project! {
    pub struct ExpectPanicFuture<T: std::future::Future> {
        context: &'static str,
        #[pin]
        future: T,
    }
}

impl<T: std::future::Future> std::future::Future for ExpectPanicFuture<T> {
    type Output = ();

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let projection = self.project();
        let future = projection.future;

        // Run code that should panic, restore hook + gdext panic printing.
        let panic = suppress_panic_log(move || {
            panic::catch_unwind(panic::AssertUnwindSafe(move || future.poll(cx)))
        });

        match panic {
            Ok(std::task::Poll::Pending) => std::task::Poll::Pending,
            Err(_) => std::task::Poll::Ready(()),
            Ok(std::task::Poll::Ready(_)) => {
                panic!(
                    "code should have panicked but did not: {}",
                    projection.context
                );
            }
        }
    }
}

pub fn expect_async_panic<T: std::future::Future>(
    context: &'static str,
    future: T,
) -> ExpectPanicFuture<T> {
    ExpectPanicFuture { context, future }
}

pub fn expect_debug_panic_or_release_ok(_context: &str, code: impl FnOnce()) {
    #[cfg(debug_assertions)]
    expect_panic(_context, code);

    #[cfg(not(debug_assertions))]
    code()
}

/// Run code asynchronously, at the very start of the next _process_ frame (before `INode::process()`).
///
/// If there is a custom `MainLoop` present, it will be run _before_ this.
///
/// Useful for assertions that run expect a `call_deferred()` or similar operation, and still want to check the result.
#[cfg(since_api = "4.2")]
#[must_use]
#[allow(dead_code)] // not yet used.
pub fn next_frame<F>(code: F) -> godot::task::TaskHandle
where
    F: FnOnce() + 'static,
{
    let tree = Engine::singleton()
        .get_main_loop()
        .unwrap()
        .cast::<SceneTree>();

    godot::task::spawn(async move {
        let _: () = tree.signals().process_frame().to_future().await;
        code();
    })
}

/// Synchronously run a thread and return result. Panics are propagated to caller thread.
#[track_caller]
pub fn quick_thread<R, F>(f: F) -> R
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let handle = std::thread::spawn(f);
    match handle.join() {
        Ok(result) => result,
        Err(panic_payload) => {
            if let Some(s) = panic_payload.downcast_ref::<&str>() {
                panic!("quick_thread panicked: {s}")
            } else if let Some(s) = panic_payload.downcast_ref::<String>() {
                panic!("quick_thread panicked: {s}")
            } else {
                panic!("quick_thread panicked with unknown type.")
            };
        }
    }
}

/// Disable printing errors from Godot. Ideally we should catch and handle errors, ensuring they happen when
/// expected. But that isn't possible, so for now we can just disable printing the error to avoid spamming
/// the terminal when tests should error.
pub fn suppress_godot_print(mut f: impl FnMut()) {
    Engine::singleton().set_print_error_messages(false);
    f();
    Engine::singleton().set_print_error_messages(true);
}

/// Some tests are disabled, as they rely on Godot checks which are only available in Debug builds.
/// See <https://github.com/godotengine/godot/issues/86264>.
pub fn runs_release() -> bool {
    !Os::singleton().is_debug_build()
}

/// Workaround for tests of the form `assert!(a == a)`.
///
/// We can't always use `assert_eq!(a, a)` because of lacking `Debug` impl.
///
/// Clippy however complains, yet the suggested `#[allow(clippy::eq_op)]` cannot be used to suppress the Clippy warning (likely a bug).
#[macro_export]
macro_rules! assert_eq_self {
    ($a:expr) => {{
        if !($a == $a) {
            panic!("assertion failed: `(a == a)`");
        }
    }};
}

pub use crate::assert_eq_self;
