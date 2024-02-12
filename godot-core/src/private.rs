/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::sync::{Arc, Mutex};

pub use crate::gen::classes::class_macros;
pub use crate::registry::{callbacks, ClassPlugin, ErasedRegisterFn, PluginItem};
pub use crate::storage::{as_storage, Storage};
pub use sys::out;

use crate::{log, sys};

// If someone forgets #[godot_api], this causes a compile error, rather than virtual functions not being called at runtime.
#[allow(non_camel_case_types)]
pub trait You_forgot_the_attribute__godot_api {}

sys::plugin_registry!(pub __GODOT_PLUGIN_REGISTRY: ClassPlugin);

pub(crate) fn iterate_plugins(mut visitor: impl FnMut(&ClassPlugin)) {
    sys::plugin_foreach!(__GODOT_PLUGIN_REGISTRY; visitor);
}

pub use crate::obj::rtti::ObjectRtti;

pub struct ClassConfig {
    pub is_tool: bool,
}

pub fn is_class_inactive(is_tool: bool) -> bool {
    if is_tool {
        return false;
    }

    // SAFETY: only invoked after global library initialization.
    let global_config = unsafe { sys::config() };
    let is_editor = || crate::engine::Engine::singleton().is_editor_hint();

    global_config.tool_only_in_editor //.
        && global_config.is_editor_or_init(is_editor)
}

pub fn print_panic(err: Box<dyn std::any::Any + Send>) {
    if let Some(s) = err.downcast_ref::<&'static str>() {
        print_panic_message(s);
    } else if let Some(s) = err.downcast_ref::<String>() {
        print_panic_message(s.as_str());
    } else {
        log::godot_error!("Rust panic of type ID {:?}", err.type_id());
    }
}

pub fn auto_init<T>(l: &mut crate::obj::OnReady<T>) {
    l.init_auto();
}

fn print_panic_message(msg: &str) {
    // If the message contains newlines, print all of the lines after a line break, and indent them.
    let lbegin = "\n  ";
    let indented = msg.replace('\n', lbegin);

    if indented.len() != msg.len() {
        log::godot_error!("Panic msg:{lbegin}{indented}");
    } else {
        log::godot_error!("Panic msg:  {msg}");
    }
}

struct GodotPanicInfo {
    line: u32,
    file: String,
    //backtrace: Backtrace, // for future use
}

/// Executes `code`. If a panic is thrown, it is caught and an error message is printed to Godot.
///
/// Returns `None` if a panic occurred, and `Some(result)` with the result of `code` otherwise.
#[must_use]
pub fn handle_panic<E, F, R, S>(error_context: E, code: F) -> Option<R>
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
        Ok(result) => Some(result),
        Err(err) => {
            // Flush, to make sure previous Rust output (e.g. test announcement, or debug prints during app) have been printed
            // TODO write custom panic handler and move this there, before panic backtrace printing
            flush_stdout();

            let guard = info.lock().unwrap();
            let info = guard.as_ref().expect("no panic info available");
            log::godot_error!(
                "Rust function panicked at {}:{}.\nContext: {}",
                info.file,
                info.line,
                error_context()
            );
            //eprintln!("Backtrace:\n{}", info.backtrace);
            print_panic(err);
            None
        }
    }
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
pub const fn is_editor_plugin<T: crate::obj::Inherits<crate::engine::EditorPlugin>>() {}
