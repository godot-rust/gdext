/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

pub use crate::gen::classes::class_macros;
pub use crate::obj::rtti::ObjectRtti;
pub use crate::registry::callbacks;
pub use crate::registry::plugin::{ClassPlugin, ErasedRegisterFn, PluginItem};
pub use crate::storage::{as_storage, Storage};
pub use sys::out;

#[cfg(feature = "trace")]
pub use crate::meta::trace;

use crate::global::godot_error;
use crate::meta::error::CallError;
use crate::meta::CallContext;
use crate::sys;
use std::sync::{atomic, Arc, Mutex};
use sys::Global;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Global variables

static CALL_ERRORS: Global<CallErrors> = Global::default();

/// Level:
/// - 0: no error printing (during `expect_panic` in test)
/// - 1: not yet implemented, but intended for `try_` function calls (which are expected to fail, so error is annoying)
/// - 2: normal printing
static ERROR_PRINT_LEVEL: atomic::AtomicU8 = atomic::AtomicU8::new(2);

sys::plugin_registry!(pub __GODOT_PLUGIN_REGISTRY: ClassPlugin);

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Call error handling

// Note: if this leads to many allocated IDs that are not removed, we could limit to 1 per thread-ID.
// Would need to check if re-entrant calls with multiple errors per thread are possible.
struct CallErrors {
    ring_buffer: Vec<Option<CallError>>,
    next_id: u8,
    generation: u16,
}

impl Default for CallErrors {
    fn default() -> Self {
        Self {
            // [None; N] requires Clone. The following is possible once MSRV lifts to 1.79:
            // ring_buffer: [const { None }; Self::MAX_ENTRIES as usize].into(),
            ring_buffer: std::iter::repeat_with(|| None)
                .take(Self::MAX_ENTRIES as usize)
                .collect(),
            next_id: 0,
            generation: 0,
        }
    }
}

impl CallErrors {
    const MAX_ENTRIES: u8 = 32;

    fn insert(&mut self, err: CallError) -> i32 {
        let id = self.next_id;

        self.next_id = self.next_id.wrapping_add(1) % Self::MAX_ENTRIES;
        if self.next_id == 0 {
            self.generation = self.generation.wrapping_add(1);
        }

        self.ring_buffer[id as usize] = Some(err);

        (self.generation as i32) << 16 | id as i32
    }

    // Returns success or failure.
    fn remove(&mut self, id: i32) -> Option<CallError> {
        let generation = (id >> 16) as u16;
        let id = id as u8;

        // If id < next_id, the generation must be the current one -- otherwise the one before.
        if id < self.next_id {
            if generation != self.generation {
                return None;
            }
        } else if generation != self.generation.wrapping_sub(1) {
            return None;
        }

        // Returns Some if there's still an entry, None if it was already removed.
        self.ring_buffer[id as usize].take()
    }
}

/// Inserts a `CallError` into a global variable and returns its ID to later remove it.
fn call_error_insert(err: CallError) -> i32 {
    // Wraps around if entire i32 is depleted. If this happens in practice (unlikely, users need to deliberately ignore errors that are printed),
    // we just overwrite the oldest errors, should still work.
    let id = CALL_ERRORS.lock().insert(err);
    id
}

pub(crate) fn call_error_remove(in_error: &sys::GDExtensionCallError) -> Option<CallError> {
    // Error checks are just quality-of-life diagnostic; do not throw panics if they fail.

    if in_error.error != sys::GODOT_RUST_CUSTOM_CALL_ERROR {
        godot_error!("Tried to remove non-godot-rust call error {in_error:?}");
        return None;
    }

    let call_error = CALL_ERRORS.lock().remove(in_error.argument);
    if call_error.is_none() {
        // Just a quality-of-life diagnostic; do not throw panics if something like this fails.
        godot_error!("Failed to remove call error {in_error:?}");
    }

    call_error
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Plugin handling

pub(crate) fn iterate_plugins(mut visitor: impl FnMut(&ClassPlugin)) {
    sys::plugin_foreach!(__GODOT_PLUGIN_REGISTRY; visitor);
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Traits and types

// If someone forgets #[godot_api], this causes a compile error, rather than virtual functions not being called at runtime.
#[allow(non_camel_case_types)]
#[diagnostic::on_unimplemented(
    message = "`impl` blocks for Godot classes require the `#[godot_api]` attribute",
    label = "missing `#[godot_api]` before `impl`",
    note = "see also: https://godot-rust.github.io/book/register/functions.html#godot-special-functions"
)]
pub trait You_forgot_the_attribute__godot_api {}

pub struct ClassConfig {
    pub is_tool: bool,
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Capability queries and internal access

pub fn auto_init<T>(l: &mut crate::obj::OnReady<T>, base: &crate::obj::Gd<crate::classes::Node>) {
    l.init_auto(base);
}

#[cfg(since_api = "4.3")]
pub unsafe fn has_virtual_script_method(
    object_ptr: sys::GDExtensionObjectPtr,
    method_sname: sys::GDExtensionConstStringNamePtr,
) -> bool {
    sys::interface_fn!(object_has_script_method)(sys::to_const_ptr(object_ptr), method_sname) != 0
}

pub fn flush_stdout() {
    use std::io::Write;
    std::io::stdout().flush().expect("flush stdout");
}

/// Ensure `T` is an editor plugin.
pub const fn is_editor_plugin<T: crate::obj::Inherits<crate::classes::EditorPlugin>>() {}

// Starting from 4.3, Godot has "runtime classes"; this emulation is no longer needed.
#[cfg(before_api = "4.3")]
pub fn is_class_inactive(is_tool: bool) -> bool {
    if is_tool {
        return false;
    }

    // SAFETY: only invoked after global library initialization.
    let global_config = unsafe { sys::config() };
    let is_editor = || crate::classes::Engine::singleton().is_editor_hint();

    global_config.tool_only_in_editor //.
        && global_config.is_editor_or_init(is_editor)
}

// Starting from 4.3, Godot has "runtime classes"; we only need to check whether editor is running.
#[cfg(since_api = "4.3")]
pub fn is_class_runtime(is_tool: bool) -> bool {
    if is_tool {
        return false;
    }

    // SAFETY: only invoked after global library initialization.
    let global_config = unsafe { sys::config() };

    // If this is not a #[class(tool)] and we only run tool classes in the editor, then don't run this in editor -> make it a runtime class.
    // If we run all classes in the editor (!tool_only_in_editor), then it's not a runtime class.
    global_config.tool_only_in_editor
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Panic handling

#[derive(Debug)]
struct GodotPanicInfo {
    line: u32,
    file: String,
    //backtrace: Backtrace, // for future use
}

pub fn extract_panic_message(err: Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = err.downcast_ref::<&'static str>() {
        s.to_string()
    } else if let Some(s) = err.downcast_ref::<String>() {
        s.clone()
    } else {
        format!("(panic of type ID {:?})", err.type_id())
    }
}

fn format_panic_message(msg: String) -> String {
    // If the message contains newlines, print all of the lines after a line break, and indent them.
    let lbegin = "\n  ";
    let indented = msg.replace('\n', lbegin);

    if indented.len() != msg.len() {
        format!("[panic]{lbegin}{indented}")
    } else {
        format!("[panic]  {msg}")
    }
}

pub fn set_error_print_level(level: u8) -> u8 {
    assert!(level <= 2);
    ERROR_PRINT_LEVEL.swap(level, atomic::Ordering::Relaxed)
}

pub(crate) fn has_error_print_level(level: u8) -> bool {
    assert!(level <= 2);
    ERROR_PRINT_LEVEL.load(atomic::Ordering::Relaxed) >= level
}

/// Executes `code`. If a panic is thrown, it is caught and an error message is printed to Godot.
///
/// Returns `Err(message)` if a panic occurred, and `Ok(result)` with the result of `code` otherwise.
///
/// In contrast to [`handle_varcall_panic`] and [`handle_ptrcall_panic`], this function is not intended for use in `try_` functions,
/// where the error is propagated as a `CallError` in a global variable.
pub fn handle_panic<E, F, R, S>(error_context: E, code: F) -> Result<R, String>
where
    E: FnOnce() -> S,
    F: FnOnce() -> R + std::panic::UnwindSafe,
    S: std::fmt::Display,
{
    handle_panic_with_print(error_context, code, has_error_print_level(1))
}

// TODO(bromeon): make call_ctx lazy-evaluated (like error_ctx) everywhere;
// or make it eager everywhere and ensure it's cheaply constructed in the call sites.
pub fn handle_varcall_panic<F, R>(
    call_ctx: &CallContext,
    out_err: &mut sys::GDExtensionCallError,
    code: F,
) where
    F: FnOnce() -> Result<R, CallError> + std::panic::UnwindSafe,
{
    let outcome: Result<Result<R, CallError>, String> =
        handle_panic_with_print(|| call_ctx, code, false);

    let call_error = match outcome {
        // All good.
        Ok(Ok(_result)) => return,

        // Call error signalled by Godot's or gdext's validation.
        Ok(Err(err)) => err,

        // Panic occurred (typically through user): forward message.
        Err(panic_msg) => CallError::failed_by_user_panic(call_ctx, panic_msg),
    };

    let error_id = report_call_error(call_error, true);

    // Abuse 'argument' field to store our ID.
    *out_err = sys::GDExtensionCallError {
        error: sys::GODOT_RUST_CUSTOM_CALL_ERROR,
        argument: error_id,
        expected: 0,
    };

    //sys::interface_fn!(variant_new_nil)(sys::AsUninit::as_uninit(ret));
}

pub fn handle_ptrcall_panic<F, R>(call_ctx: &CallContext, code: F)
where
    F: FnOnce() -> R + std::panic::UnwindSafe,
{
    let outcome: Result<R, String> = handle_panic_with_print(|| call_ctx, code, false);

    let call_error = match outcome {
        // All good.
        Ok(_result) => return,

        // Panic occurred (typically through user): forward message.
        Err(panic_msg) => CallError::failed_by_user_panic(call_ctx, panic_msg),
    };

    let _id = report_call_error(call_error, false);
}

fn report_call_error(call_error: CallError, track_globally: bool) -> i32 {
    // Print failed calls to Godot's console.
    // TODO Level 1 is not yet set, so this will always print if level != 0. Needs better logic to recognize try_* calls and avoid printing.
    // But a bit tricky with multiple threads and re-entrancy; maybe pass in info in error struct.
    if has_error_print_level(2) {
        godot_error!("{call_error}");
    }

    // Once there is a way to auto-remove added errors, this could be always true.
    if track_globally {
        call_error_insert(call_error)
    } else {
        0
    }
}

fn handle_panic_with_print<E, F, R, S>(error_context: E, code: F, print: bool) -> Result<R, String>
where
    E: FnOnce() -> S,
    F: FnOnce() -> R + std::panic::UnwindSafe,
    S: std::fmt::Display,
{
    let info: Arc<Mutex<Option<GodotPanicInfo>>> = Arc::new(Mutex::new(None));

    // Back up previous hook, set new one
    let prev_hook = std::panic::take_hook();
    {
        let info = info.clone();
        std::panic::set_hook(Box::new(move |panic_info| {
            if let Some(location) = panic_info.location() {
                *info.lock().unwrap() = Some(GodotPanicInfo {
                    file: location.file().to_string(),
                    line: location.line(),
                    //backtrace: Backtrace::capture(),
                });
            } else {
                eprintln!("panic occurred, but can't get location information");
            }
        }));
    }

    // Run code that should panic, restore hook
    let panic = std::panic::catch_unwind(code);
    std::panic::set_hook(prev_hook);

    match panic {
        Ok(result) => Ok(result),
        Err(err) => {
            // Flush, to make sure previous Rust output (e.g. test announcement, or debug prints during app) have been printed
            // TODO write custom panic handler and move this there, before panic backtrace printing
            flush_stdout();

            let guard = info.lock().unwrap();
            let info = guard.as_ref().expect("no panic info available");

            if print {
                godot_error!(
                    "Rust function panicked at {}:{}.\n  Context: {}",
                    info.file,
                    info.line,
                    error_context()
                );
                //eprintln!("Backtrace:\n{}", info.backtrace);
            }

            let msg = extract_panic_message(err);
            let msg = format_panic_message(msg);

            if print {
                godot_error!("{msg}");
            }

            Err(msg)
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::{CallError, CallErrors};
    use crate::meta::CallContext;

    fn make(index: usize) -> CallError {
        let method_name = format!("method_{index}");
        let ctx = CallContext::func("Class", &method_name);
        CallError::failed_by_user_panic(&ctx, "some panic reason".to_string())
    }

    #[test]
    fn test_call_errors() {
        let mut store = CallErrors::default();

        let mut id07 = 0;
        let mut id13 = 0;
        let mut id20 = 0;
        for i in 0..24 {
            let id = store.insert(make(i));
            match i {
                7 => id07 = id,
                13 => id13 = id,
                20 => id20 = id,
                _ => {}
            }
        }

        let e = store.remove(id20).expect("must be present");
        assert_eq!(e.method_name(), "method_20");

        let e = store.remove(id20);
        assert!(e.is_none());

        for i in 24..CallErrors::MAX_ENTRIES as usize {
            store.insert(make(i));
        }
        for i in 0..10 {
            store.insert(make(i));
        }

        let e = store.remove(id07);
        assert!(e.is_none(), "generation overwritten");

        let e = store.remove(id13).expect("generation not yet overwritten");
        assert_eq!(e.method_name(), "method_13");
    }
}
