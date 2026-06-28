/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Printing and logging functionality.

// Thread safety (applies to every print macro, to `print_custom`, and to the `print`/`printerr`/`str`/... utility functions):
//
// The whole print group can be called from any thread on standard Desktop builds. We therefore route it through the thread-safe binding
// accessors (`utility_function_table_thread_safe()` for the utility functions, `sys::thread_safe()` for the error/warning interface functions)
// rather than the main-thread-asserting default.
//
// Call chain from a worker thread: macro -> `global::print()` / `print_custom()` -> Godot `OS::print()` -> active `Logger` -> `CompositeLogger`
// -> sub-loggers. `OS::print()` runs *outside* Godot's `_global_lock`, so line atomicity is not guaranteed; the C++ code below is nonetheless
// safe against memory corruption because each backend ends up in a libc call that holds a per-`FILE` lock:
// - `StdLogger` (always installed): `vprintf`/`vfprintf` -> glibc per-`FILE` lock. No data race on the C runtime; worst case is interleaved
//   lines (cosmetic), never memory-unsafe.
// - `RotatedFileLogger` (installed by `--log-file` or the `enable_file_logging` setting, default-on): per line it does `file->store_buffer(...)`
//   -> `FileAccessUnix::store_buffer` = `fwrite` (+ `fflush`), again under glibc's per-`FILE` lock, so writes are serialized, not torn. The
//   shared ANSI-strip `RegEx` hit on every line is used in PCRE2's documented thread-safe pattern (read-only compiled pattern, per-call match
//   data and match context allocated as locals), and with `--log-file` there is no per-call rotation/size bookkeeping. Verified empirically:
//   16 threads x 20000 iterations, 13 headless runs, produced a deterministic 560005-line log with no crash and no torn writes.
//
// Two *formal* C++ data races remain but are benign on real glibc targets: the non-atomic `CoreGlobals` print flags (an aligned `bool`
// load/store is atomic on every real target) and the unlocked `file` access in `logv()` (serialized by the libc `FILE` lock underneath). A
// thread sanitizer flags them; a running binary does not crash.
//
// Caveat: the file-logger safety relies on the `FileAccess` backend happening to lock internally. `FileAccessUnix` does (via libc); a custom or
// exotic-platform `FileAccess` that buffers in its own non-thread-safe state could turn the formal `file` race into real torn writes. We accept
// this -- it is not the default desktop/headless configuration -- rather than gate printing, which is a core cross-thread debugging tool.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use crate::builtin::Variant;
use crate::classes::Engine;
use crate::obj::Singleton;
use crate::sys;

// https://stackoverflow.com/a/40234666
#[macro_export]
#[doc(hidden)]
macro_rules! inner_function {
    () => {{
        fn f() {}
        fn type_name_of<T>(_: T) -> &'static str {
            std::any::type_name::<T>()
        }
        let name = type_name_of(f);
        name.strip_suffix("::f").unwrap()
    }};
}

#[macro_export]
#[doc(hidden)]
macro_rules! inner_godot_msg {
    ($level:expr; $fmt:literal $(, $args:expr)* $(,)?) => {
        {
            let description = format!($fmt $(, $args)*);
            $crate::global::print_custom($crate::global::PrintRecord {
                level: $level,
                message: &description,
                rationale: None,
                source: Some($crate::global::PrintSource {
                    function: $crate::inner_function!(),
                    file: file!(),
                    line: line!(),
                }),
                editor_notify: false,
            });
        }
    };
}

/// Severity level for [`print_custom()`].
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
#[non_exhaustive]
pub enum PrintLevel {
    /// Plain message. Used by [`godot_print!`][crate::global::godot_print].
    ///
    /// Godot's GDExtension API has no info-level print function that accepts a source location, so if a location is provided, godot-rust
    /// will append "at:" information to the message.
    Info,

    /// Warning. Used by [`godot_warn!`][crate::global::godot_warn].
    Warn,

    /// Error. Used by [`godot_error!`][crate::global::godot_error].
    Error,

    /// Script error (rarely needed in Rust). Used by [`godot_script_error!`][crate::global::godot_script_error].
    ScriptError,
}

impl PrintLevel {
    /// Returns the upper-case prefix used by Godot to print on stdout/stderr.
    ///
    /// E.g. `Some("WARNING")` for `Warn` and `None` (no prefix) for `Info`.
    pub fn godot_title(self) -> Option<&'static str> {
        match self {
            Self::Warn => Some("WARNING"),
            Self::Error => Some("ERROR"),
            Self::ScriptError => Some("SCRIPT ERROR"),
            Self::Info => None,
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Source location associated with a [`PrintRecord`].
///
/// Can be built directly from explicit values, or via [`from_location()`][Self::from_location] from a [`std::panic::Location`].
///
/// If any of the string-based fields contain interior null bytes (`\0`), the string may be cut off at that point.
#[derive(Copy, Clone, Debug)]
pub struct PrintSource<'a> {
    pub function: &'a str,
    pub file: &'a str,
    pub line: u32,
}

impl<'a> PrintSource<'a> {
    /// Build a `PrintSource` from a [`std::panic::Location`] and a function name.
    ///
    /// Function name must be supplied separately, since [`Location`][std::panic::Location] does not capture it.
    pub fn from_location(location: &'a std::panic::Location<'a>, function: &'a str) -> Self {
        Self {
            function,
            file: location.file(),
            line: location.line(),
        }
    }

    /// Build a `PrintSource` from the caller's location, with an empty function name.
    ///
    /// Uses [`std::panic::Location::caller()`]; place `#[track_caller]` on intermediate functions to forward the caller through.
    #[track_caller]
    pub fn caller() -> Self {
        Self::from_location(std::panic::Location::caller(), "")
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Log record passed to [`print_custom()`].
///
/// Allows callers to provide an *explicit* source location, rather than the call site of the print macro.
/// See also the [`Logger`][crate::classes::ILogger] interface for intercepting printed messages.
///
/// If any of the string-based fields contain interior null bytes (`\0`), the string may be cut off at that point.
#[derive(Copy, Clone, Debug)]
pub struct PrintRecord<'a> {
    /// Severity level.
    pub level: PrintLevel,

    /// Primary message. Shown in the editor's debugger panel (warn/error) or output panel (info), and on the OS terminal.
    pub message: &'a str,

    /// Optional secondary message, displayed separately in editor UI for warn/error/script-error.
    ///
    /// For [`PrintLevel::Info`], it is appended to the description as `"description: message"`.
    pub rationale: Option<&'a str>,

    /// Source location.
    ///
    /// - For warn/error/script-error: if `None`, falls back to [`PrintSource::caller()`] (function name is empty).
    /// - For [`PrintLevel::Info`]: if `Some`, formatted into the message; if `None`, no source suffix is appended.
    pub source: Option<PrintSource<'a>>,

    /// Whether to create a toast notification in the editor. Ignored for [`PrintLevel::Info`].
    pub editor_notify: bool,
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Low-level printing of log messages with full control over level, source location and editor toast.
///
/// Most users should prefer the [`godot_print!`][crate::global::godot_print], [`godot_warn!`][crate::global::godot_warn] and
/// [`godot_error!`][crate::global::godot_error] macros. `print_custom()` is intended for low-level configurability or integration with crates
/// like `tracing` or `log`.
///
/// See [`PrintRecord`] and [`PrintLevel`] for routing and source-location behavior. Due to the use of C-strings, if any of the string fields in
/// `PrintRecord` or [`PrintSource`] have a nul byte (`\0`) in the middle, the printed text will be cut off at that nul byte. Consider this
/// when working with user-provided texts (e.g. for logging).
///
/// # Thread safety
/// Safe to call from any thread on standard desktop builds; output from concurrent threads may interleave between lines, which is cosmetic. See
/// the thread-safety note at the print macros (this module) for the per-backend C++ rationale and the one caveat (custom non-locking
/// `FileAccess` log backends).
#[track_caller]
pub fn print_custom(record: PrintRecord<'_>) {
    // Engine not yet loaded -- fall back to stderr.
    if !sys::is_godot_initialized() {
        let level = record
            .level
            .godot_title()
            .map_or(String::new(), |t| format!("{t}:"));

        match record.rationale {
            Some(msg) => eprintln!("{level}{} ({msg})", record.message),
            None => eprintln!("{level}{}", record.message),
        }
        return;
    }

    if record.level == PrintLevel::Info {
        print_info(record.message, record.rationale, record.source);
        return;
    }

    // Default location from caller, when source is not given.
    let source = record.source.unwrap_or_else(PrintSource::caller);
    let PrintSource {
        function,
        file,
        line,
    } = source;

    // Null-terminate strings (Godot expects C strings).
    let desc_nul = format!("{}\0", record.message);
    let func_nul = format!("{function}\0");
    let file_nul = format!("{file}\0");
    let msg_nul = record.rationale.map(|m| format!("{m}\0"));
    let editor_notify = sys::conv::bool_to_sys(record.editor_notify);

    let desc_ptr = sys::c_str_from_str(&desc_nul);
    let func_ptr = sys::c_str_from_str(&func_nul);
    let file_ptr = sys::c_str_from_str(&file_nul);
    let line = line as i32;

    // SAFETY: engine initialized; interface functions valid; pointers live for the call duration. The print/error functions route to Godot's
    // logger, which is thread-safe in practice (see the note at the print macros), so they go through the thread-safe accessor rather than the
    // main-thread one -- allowing prints from worker threads.
    unsafe {
        let interface = sys::thread_safe();
        if let Some(msg_z) = &msg_nul {
            let godot_fn = match record.level {
                PrintLevel::Warn => interface.print_warning_with_message,
                PrintLevel::Error => interface.print_error_with_message,
                PrintLevel::ScriptError => interface.print_script_error_with_message,
                PrintLevel::Info => unreachable!(),
            };
            godot_fn(
                desc_ptr,
                sys::c_str_from_str(msg_z),
                func_ptr,
                file_ptr,
                line,
                editor_notify,
            );
        } else {
            let godot_fn = match record.level {
                PrintLevel::Warn => interface.print_warning,
                PrintLevel::Error => interface.print_error,
                PrintLevel::ScriptError => interface.print_script_error,
                PrintLevel::Info => unreachable!(),
            };
            godot_fn(desc_ptr, func_ptr, file_ptr, line, editor_notify);
        }
    }
}

fn print_info(description: &str, message: Option<&str>, source: Option<PrintSource<'_>>) {
    // `format!` pre-sizes the buffer to fit; each branch is a single allocation with no realloc.
    let full = match (message, source) {
        (Some(m), None) => format!("{description}: {m}"),
        (Some(m), Some(s)) => {
            format!(
                "{description}: {m}\n\tat: {} ({}:{})",
                s.function, s.file, s.line
            )
        }
        (None, Some(s)) => format!(
            "{description}\n\tat: {} ({}:{})",
            s.function, s.file, s.line
        ),
        (None, None) => description.to_string(),
    };

    // Route through the thread-safe path: building+dropping a `Variant` here would hit the main-thread-gated `Drop`, but `print_custom` is callable
    // from any thread.
    __threadsafe_print(full, false);
}

/// Thread-safe printing backend.
///
/// The `print`/`print_rich` utility functions are themselves thread-safe, but they take a `Variant`, whose general destructor requires main
/// thread (a `Variant` may hold an `Object`).
#[doc(hidden)]
pub fn __threadsafe_print(message: String, rich: bool) {
    let variant = Variant::from(message);

    if rich {
        crate::global::print_rich(std::slice::from_ref(&variant));
    } else {
        crate::global::print(std::slice::from_ref(&variant));
    }

    // SAFETY: variant is GString payload, which is Send and thus safe to destroy across threads.
    unsafe { variant.destroy_unchecked_thread() };
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Print macros

/// Pushes a warning message to Godot's built-in debugger and to the OS terminal.
///
/// # See also
/// [`godot_print!`](macro.godot_print.html) and [`godot_error!`](macro.godot_error.html).
///
/// Related to the utility function [`global::push_warning()`](crate::global::push_warning).
///
/// _Godot equivalent: [`@GlobalScope.push_warning()`](https://docs.godotengine.org/en/stable/classes/class_@globalscope.html#class-globalscope-method-push-warning)_.
#[macro_export]
macro_rules! godot_warn {
    ($fmt:literal $(, $args:expr_2021)* $(,)?) => {
        $crate::inner_godot_msg!($crate::global::PrintLevel::Warn; $fmt $(, $args)*);
    };
}

/// Pushes an error message to Godot's built-in debugger and to the OS terminal.
///
/// # See also
/// [`godot_print!`](macro.godot_print.html) and [`godot_warn!`](macro.godot_warn.html).
/// For script errors (less relevant in Rust), use [`godot_script_error!`](macro.godot_script_error.html).
///
/// Related to the utility function [`global::push_error()`][crate::global::push_error].
///
/// _Godot equivalent: [`@GlobalScope.push_error()`](https://docs.godotengine.org/en/stable/classes/class_@globalscope.html#class-globalscope-method-push-error)_.
#[macro_export]
macro_rules! godot_error {
    ($fmt:literal $(, $args:expr_2021)* $(,)?) => {
        $crate::inner_godot_msg!($crate::global::PrintLevel::Error; $fmt $(, $args)*);
    };
}

/// Logs a script error to Godot's built-in debugger and to the OS terminal.
///
/// This is rarely needed in Rust; script errors are typically emitted by the GDScript parser.
///
/// # See also
/// [`godot_error!`](macro.godot_error.html) for a general error message.
///
///
#[macro_export]
macro_rules! godot_script_error {
    ($fmt:literal $(, $args:expr_2021)* $(,)?) => {
        $crate::inner_godot_msg!($crate::global::PrintLevel::ScriptError; $fmt $(, $args)*);
    };
}

/// Prints to the Godot console.
///
/// Automatically appends a newline character at the end of the message.
///
/// Used exactly like standard [`println!`]:
/// ```no_run
/// use godot::global::godot_print;
///
/// let version = 4;
/// godot_print!("Hello, Godot {version}!");
/// ```
///
/// # See also
/// [`godot_print_rich!`](macro.godot_print_rich.html) for a slower alternative that supports BBCode, color and URL tags.
/// To print Godot errors and warnings, use [`godot_error!`](macro.godot_error.html) and [`godot_warn!`](macro.godot_warn.html), respectively.
///
/// This uses the underlying [`global::print()`][crate::global::print] function, which takes a variable-length slice of variants.
///
/// _Godot equivalent: [`@GlobalScope.print()`](https://docs.godotengine.org/en/stable/classes/class_@globalscope.html#class-globalscope-method-print)_.
#[macro_export]
macro_rules! godot_print {
    ($fmt:literal $(, $args:expr_2021)* $(,)?) => {
        $crate::global::__threadsafe_print(format!($fmt $(, $args)*), false)
    };
}

/// Prints to the Godot console. Supports BBCode, color and URL tags.
///
/// Slower than [`godot_print!`](macro.godot_print_rich.html).
///
/// _Godot equivalent: [`@GlobalScope.print_rich()`](https://docs.godotengine.org/en/stable/classes/class_@globalscope.html#class-globalscope-method-print-rich)_.
#[macro_export]
macro_rules! godot_print_rich {
    ($fmt:literal $(, $args:expr_2021)* $(,)?) => {
        $crate::global::__threadsafe_print(format!($fmt $(, $args)*), true)
    };
}

/// Concatenates format-style arguments into a `GString`.
///
/// Works similar to Rust's standard [`format!`] macro but returns a Godot `GString`.
///
/// # Example
/// ```no_run
/// use godot::builtin::GString;
/// use godot::global::godot_str;
///
/// let name = "Player";
/// let score = 100;
/// let message: GString = godot_str!("The {name} scored {score} points!");
/// ```
///
/// # See also
/// This macro uses the underlying [`global::str()`][crate::global::str] function, which takes a variable-length slice of variants.
///
/// _Godot equivalent: [`@GlobalScope.str()`](https://docs.godotengine.org/en/stable/classes/class_@globalscope.html#class-globalscope-method-str)_.
#[macro_export]
macro_rules! godot_str {
    ($fmt:literal $(, $args:expr_2021)* $(,)?) => {
        $crate::global::str(&[
            $crate::builtin::Variant::from(
                format!($fmt $(, $args)*)
            )
        ])
    };
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Print suppress logic

/// Number of active [`suppress_godot_errors()`] scopes; the flag toggles on the `0 <-> 1` transitions. For multithreading robustness.
static SUPPRESS_DEPTH: AtomicU32 = AtomicU32::new(0);

/// Value of `Engine::is_printing_error_messages()` captured when the outermost scope began, restored when it exits.
static SUPPRESS_ORIG_STATE: AtomicBool = AtomicBool::new(true);

/// RAII guard returned by [`suppress_godot_errors()`]; restores Godot error printing once the outermost scope drops.
#[doc(hidden)]
#[must_use = "keep RAII guard alive, e.g. `let _guard = suppress_godot_errors();`"]
pub struct SuppressGuard {
    _private: (),
}

impl Drop for SuppressGuard {
    fn drop(&mut self) {
        let prev = SUPPRESS_DEPTH.fetch_sub(1, Ordering::AcqRel);
        sys::strict_assert!(prev >= 1, "suppress_godot_errors: depth underflow");
        if prev == 1 {
            let restore = SUPPRESS_ORIG_STATE.load(Ordering::Acquire);
            Engine::singleton().set_print_error_messages(restore);
        }
    }
}

/// Disables Godot's console error printing until the returned [`SuppressGuard`] drops.
///
/// Silences expected, Rust-handled failures (e.g. [`try_load()`][crate::tools::try_load] returning `Err`). Panic-safe via the guard's
/// `Drop`. The flag is process-wide: nested/concurrent scopes only toggle at the `0 <-> 1` boundary, and the outermost scope restores the
/// prior state (races are self-healing -- worst case a transient cosmetic over/under-print). Genuine errors on other threads are hidden
/// while any scope is active, so keep the suppressed region small.
///
/// Note: [`Engine::set_print_error_messages()`] uses the main-thread-asserting binding; off-thread use would be UB.
#[doc(hidden)]
pub fn suppress_godot_errors() -> SuppressGuard {
    if SUPPRESS_DEPTH.fetch_add(1, Ordering::AcqRel) == 0 {
        SUPPRESS_ORIG_STATE.store(
            Engine::singleton().is_printing_error_messages(),
            Ordering::Release,
        );
        Engine::singleton().set_print_error_messages(false);
    }
    SuppressGuard { _private: () }
}
