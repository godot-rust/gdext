/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// If running in tests, a lot of symbols are unused or panic early
#![cfg_attr(feature = "unit-test", allow(unreachable_code, unused))]

// More test hacks...
//
// Technically, `cargo test -p godot-core` *could* be supported by this abomination:
//   #[cfg(not(any(test, doctest, feature = "unit-test"))]
// which would be necessary because `cargo test` runs both test/doctest, and downstream crates may need the feature as
// workaround https://github.com/rust-lang/rust/issues/59168#issuecomment-962214945. However, this *also* does not work,
// as #[cfg(doctest)] is currently near-useless for conditional compilation: https://github.com/rust-lang/rust/issues/67295.
// Yet even then, our compile error here is only one of many, as the compiler tries to build doctest without hitting this.
#[cfg(all(test, not(feature = "unit-test")))]
compile_error!("Running `cargo test` requires `--features unit-test`.");

// ----------------------------------------------------------------------------------------------------------------------------------------------

mod registry;
mod storage;

pub mod bind;
pub mod builder;
pub mod builtin;
pub mod init;
pub mod log;
pub mod macros;
pub mod obj;

pub use godot_ffi as sys;
pub use registry::*;

#[cfg(not(any(test, feature = "unit-test")))]
pub mod engine;

// Output of generated code. Mimics the file structure, symbols are re-exported.
#[rustfmt::skip]
#[allow(unused_imports, dead_code, non_upper_case_globals, non_snake_case)]
mod gen;

#[cfg(feature = "unit-test")]
mod test_stubs;
#[cfg(feature = "unit-test")]
pub use test_stubs::*;

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
            log::godot_error!("rust-panic of type ID {:?}", err.type_id());
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
