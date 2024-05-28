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
use std::collections::HashMap;
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
#[derive(Default)]
struct CallErrors {
    map: HashMap<i32, CallError>,
    next_id: i32,
}

impl CallErrors {
    fn insert(&mut self, err: CallError) -> i32 {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);

        self.map.insert(id, err);
        id
    }

    // Returns success or failure.
    fn remove(&mut self, id: i32) -> Option<CallError> {
        self.map.remove(&id)
    }
}

fn call_error_insert(err: CallError, out_error: &mut sys::GDExtensionCallError) {
    // Wraps around if entire i32 is depleted. If this happens in practice (unlikely, users need to deliberately ignore errors that are printed),
    // we just overwrite oldest errors, should still work.
    let id = CALL_ERRORS.lock().insert(err);

    // Abuse field to store our ID.
    out_error.argument = id;
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

pub fn auto_init<T>(l: &mut crate::obj::OnReady<T>) {
    l.init_auto();
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
pub fn handle_panic<E, F, R, S>(error_context: E, code: F) -> Result<R, String>
where
    E: FnOnce() -> S,
    F: FnOnce() -> R + std::panic::UnwindSafe,
    S: std::fmt::Display,
{
    handle_panic_with_print(error_context, code, has_error_print_level(1))
}

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

    // Print failed calls to Godot's console.
    // TODO Level 1 is not yet set, so this will always print if level != 0. Needs better logic to recognize try_* calls and avoid printing.
    // But a bit tricky with multiple threads and re-entrancy; maybe pass in info in error struct.
    if has_error_print_level(2) {
        godot_error!("{call_error}");
    }

    out_err.error = sys::GODOT_RUST_CUSTOM_CALL_ERROR;
    call_error_insert(call_error, out_err);

    //sys::interface_fn!(variant_new_nil)(sys::AsUninit::as_uninit(ret));
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
