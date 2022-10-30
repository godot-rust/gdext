/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

mod registry;
mod storage;

pub mod bind;
pub mod builder;
pub mod builtin;
pub mod init;
pub mod log;
pub mod macros;
pub mod obj;

pub use registry::*;

pub use godot_ffi as sys;

mod gen {
    #[allow(unused_imports, dead_code)]
    pub(crate) mod classes {
        // Path to core/classes/obj
        // Do not write macro for this, as it confuses IDEs -- just search&replace
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../target/godot-gen/core/classes/obj"
        ));
    }

    pub mod utilities {
        // Path to core/utilities.rs
        // Do not write macro for this, as it confuses IDEs -- just search&replace
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../target/godot-gen/core/utilities.rs"
        ));
    }

    pub mod central_core {
        // Path to core/utilities.rs
        // Do not write macro for this, as it confuses IDEs -- just search&replace
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../target/godot-gen/core/central.rs"
        ));
    }
}

/// Godot engine classes and methods.
pub mod engine {
    pub use super::gen::central_core::global;
    pub use super::gen::classes::*;
    pub use super::gen::utilities;
}

#[doc(hidden)]
pub mod private {
    pub use crate::builtin::func_callbacks;
    pub use crate::gen::classes::class_macros;
    pub use crate::registry::{callbacks, ClassPlugin, ErasedRegisterFn, PluginComponent};
    pub use crate::storage::as_storage;
    pub use crate::{gdext_register_method, gdext_register_method_inner};

    use crate::{log, sys};

    sys::plugin_registry!(__GODOT_PLUGIN_REGISTRY: ClassPlugin);

    pub(crate) fn iterate_plugins(mut visitor: impl FnMut(&ClassPlugin)) {
        sys::plugin_foreach!(__GODOT_PLUGIN_REGISTRY; visitor);
    }

    pub fn print_panic(err: Box<dyn std::any::Any + Send>) {
        if let Some(s) = err.downcast_ref::<&'static str>() {
            log::godot_error!("rust-panic:  {}", s);
        } else if let Some(s) = err.downcast_ref::<String>() {
            log::godot_error!("rust-panic:  {}", s);
        } else {
            log::godot_error!("rust-panic of type ID {:?}", (err.type_id()));
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
