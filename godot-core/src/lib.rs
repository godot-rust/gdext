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
pub mod macros;
pub mod obj;
pub mod traits;

pub use registry::*;

use godot_ffi as sys;

mod gen {
    #[allow(unused_imports, dead_code)]
    pub(crate) mod classes {
        // Path to core/classes/mod.rs
        // Do not write macro for this, as it confuses IDEs -- just search&replace
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../target/godot-gen/core/classes/mod.rs"
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

    // #[path = "../../../godot-ffi/src/gen/core/central.rs"]
    // pub mod central_core;
}

pub mod api {
    pub use super::gen::classes::*;
    pub use super::gen::utilities;
}

#[doc(hidden)]
pub mod private {
    pub use crate::builtin::func_callbacks;
    pub use crate::registry::{callbacks, ClassPlugin, ErasedRegisterFn, PluginComponent};
    pub use crate::storage::as_storage;

    godot_ffi::plugin_registry!(godot_core_REGISTRY: ClassPlugin);

    pub(crate) fn iterate_plugins(mut visitor: impl FnMut(&ClassPlugin)) {
        godot_ffi::plugin_foreach!(godot_core_REGISTRY; visitor);
    }

    pub fn print_panic(err: Box<dyn std::any::Any + Send>) {
        if let Some(s) = err.downcast_ref::<&'static str>() {
            gdext_print_error!("rust-panic:  {}", s);
        } else if let Some(s) = err.downcast_ref::<String>() {
            gdext_print_error!("rust-panic:  {}", s);
        } else {
            // FIXME expr needs to be escaped
            gdext_print_error!("rust-panic of type ID {:?}", (err.type_id()));
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
