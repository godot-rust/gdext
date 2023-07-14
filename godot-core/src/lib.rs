/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

mod registry;
mod storage;

pub mod builder;
pub mod builtin;
pub mod init;
pub mod log;
pub mod obj;
pub mod property;

pub use godot_ffi as sys;
#[doc(hidden)]
pub use godot_ffi::out;
pub use registry::*;

/// Maps the Godot class API to Rust.
///
/// This module contains the following symbols:
/// * Classes: `CanvasItem`, etc.
/// * Virtual traits: `CanvasItemVirtual`, etc.
/// * Enum/flag modules: `canvas_item`, etc.
///
/// Noteworthy sub-modules are:
/// * [`notify`][crate::engine::notify]: all notification types, used when working with the virtual callback to handle lifecycle notifications.
/// * [`global`][crate::engine::global]: global enums not belonging to a specific class.
/// * [`utilities`][crate::engine::utilities]: utility methods that are global in Godot.
pub mod engine;

// Output of generated code. Mimics the file structure, symbols are re-exported.
#[rustfmt::skip]
#[allow(unused_imports, dead_code, non_upper_case_globals, non_snake_case)]
#[allow(clippy::too_many_arguments, clippy::let_and_return, clippy::new_ret_no_self)]
#[allow(clippy::wrong_self_convention)] // to_string() is const
#[allow(clippy::upper_case_acronyms)] // TODO remove this line once we transform names
#[allow(unreachable_code, clippy::unimplemented)] // TODO remove once #153 is implemented
mod gen;

#[doc(hidden)]
pub mod private {
    // If someone forgets #[godot_api], this causes a compile error, rather than virtual functions not being called at runtime.
    #[allow(non_camel_case_types)]
    pub trait You_forgot_the_attribute__godot_api {}
    pub use crate::property::Cannot_export_without_godot_api_impl;

    use std::sync::{Arc, Mutex};

    pub use crate::gen::classes::class_macros;
    pub use crate::registry::{callbacks, ClassPlugin, ErasedRegisterFn, PluginComponent};
    pub use crate::storage::as_storage;
    pub use godot_ffi::out;

    use crate::{log, sys};

    sys::plugin_registry!(pub __GODOT_PLUGIN_REGISTRY: ClassPlugin);

    pub(crate) fn iterate_plugins(mut visitor: impl FnMut(&ClassPlugin)) {
        sys::plugin_foreach!(__GODOT_PLUGIN_REGISTRY; visitor);
    }

    fn print_panic(err: Box<dyn std::any::Any + Send>) {
        if let Some(s) = err.downcast_ref::<&'static str>() {
            print_panic_message(s);
        } else if let Some(s) = err.downcast_ref::<String>() {
            print_panic_message(s.as_str());
        } else {
            log::godot_error!("Rust panic of type ID {:?}", err.type_id());
        }
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
    }

    /// Executes `code`. If a panic is thrown, it is caught and an error message is printed to Godot.
    ///
    /// Returns `None` if a panic occurred, and `Some(result)` with the result of `code` otherwise.
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
                    });
                } else {
                    println!("panic occurred but can't get location information...");
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
                    "Rust function panicked in file {} at line {}. Context: {}",
                    info.file,
                    info.line,
                    error_context()
                );
                print_panic(err);
                None
            }
        }
    }

    pub fn flush_stdout() {
        use std::io::Write;
        std::io::stdout().flush().expect("flush stdout");
    }
}

macro_rules! generate_gdextension_api_version {
    (
        $(
            ($name:ident, $gdextension_api:ident) => {
                $($version:literal, )*
            }
        ),* $(,)?
    ) => {
        $(
            $(
                #[cfg($gdextension_api = $version)]
                #[allow(dead_code)]
                const $name: &str = $version;
            )*
        )*
    };
}

// If multiple gdextension_api_version's are found then this will generate several structs with the same
// name, causing a compile error.
//
// This includes all versions we're developing for, including unreleased future versions.
generate_gdextension_api_version!(
    (GDEXTENSION_EXACT_API, gdextension_exact_api) => {
        "4.0",
        "4.0.1",
        "4.0.2",
        "4.0.3",
        "4.1",
    },
    (GDEXTENSION_API, gdextension_minor_api) => {
        "4.0",
        "4.1",
    },
);
