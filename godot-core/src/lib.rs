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
pub mod macros;
pub mod obj;

pub use godot_ffi as sys;
pub use registry::*;

pub mod engine;

// Output of generated code. Mimics the file structure, symbols are re-exported.
#[rustfmt::skip]
#[allow(unused_imports, dead_code, non_upper_case_globals, non_snake_case, clippy::too_many_arguments, clippy::let_and_return, clippy::new_ret_no_self)]
#[allow(clippy::upper_case_acronyms)] // TODO remove this line once we transform names
#[allow(clippy::wrong_self_convention)] // TODO remove once to_string is const
mod gen;

#[doc(hidden)]
pub mod private {
    // If someone forgets #[godot_api], this causes a compile error, rather than virtual functions not being called at runtime.
    #[allow(non_camel_case_types)]
    pub trait You_forgot_the_attribute__godot_api {}

    pub use crate::gen::classes::class_macros;
    pub use crate::registry::{callbacks, ClassPlugin, ErasedRegisterFn, PluginComponent};
    pub use crate::storage::as_storage;
    pub use crate::{
        gdext_register_method, gdext_register_method_inner, gdext_virtual_method_callback,
    };

    use crate::{log, sys};

    sys::plugin_registry!(pub __GODOT_PLUGIN_REGISTRY: ClassPlugin);

    pub(crate) fn iterate_plugins(mut visitor: impl FnMut(&ClassPlugin)) {
        sys::plugin_foreach!(__GODOT_PLUGIN_REGISTRY; visitor);
    }

    fn print_panic(err: Box<dyn std::any::Any + Send>) {
        if let Some(s) = err.downcast_ref::<&'static str>() {
            log::godot_error!("Panic msg:  {s}");
        } else if let Some(s) = err.downcast_ref::<String>() {
            log::godot_error!("Panic msg:  {s}");
        } else {
            log::godot_error!("Rust panic of type ID {:?}", err.type_id());
        }
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
        match std::panic::catch_unwind(code) {
            Ok(result) => Some(result),
            Err(err) => {
                log::godot_error!("Rust function panicked. Context: {}", error_context());
                print_panic(err);
                None
            }
        }
    }
}

#[cfg(feature = "trace")]
#[macro_export]
macro_rules! out {
    ()                          => (eprintln!());
    ($fmt:literal)              => (eprintln!($fmt));
    ($fmt:literal, $($arg:tt)*) => (eprintln!($fmt, $($arg)*));
}

#[cfg(not(feature = "trace"))]
// TODO find a better way than sink-writing to avoid warnings, #[allow(unused_variables)] doesn't work
#[macro_export]
macro_rules! out {
    ()                          => ({});
    ($fmt:literal)              => ({ use std::io::{sink, Write}; let _ = write!(sink(), $fmt); });
    ($fmt:literal, $($arg:tt)*) => ({ use std::io::{sink, Write}; let _ = write!(sink(), $fmt, $($arg)*); };)
}
