/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// Note that a lot of those are public, but the godot crate still has the final say on what it wants to re-export.
// Doing fine-grained visibility restrictions on every level is a useless maintenance chore.
pub mod builder;
pub mod builtin;
pub mod classes;
pub mod global;
pub mod init;
pub mod obj;
pub mod registry;
pub mod tools;

mod storage;
pub use godot_ffi as sys;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Generated code

// Output of generated code. Mimics the file structure, symbols are re-exported.
#[rustfmt::skip]
#[allow(unused_imports, dead_code, non_upper_case_globals, non_snake_case)]
#[allow(clippy::too_many_arguments, clippy::let_and_return, clippy::new_ret_no_self)]
#[allow(clippy::let_unit_value)] // let args = ();
#[allow(clippy::wrong_self_convention)] // to_string() is const
#[allow(clippy::upper_case_acronyms)] // TODO remove this line once we transform names
#[allow(unreachable_code, clippy::unimplemented)] // TODO remove once #153 is implemented
mod gen {
    include!(concat!(env!("OUT_DIR"), "/mod.rs"));
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// API version check

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
        "4.0.4",
        "4.1",
        "4.1.1",
    },
    (GDEXTENSION_API, gdextension_minor_api) => {
        "4.0",
        "4.1",
    },
);

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Hidden but accessible symbols

/// Module which is used for deprecated warnings. It stays even if there is nothing currently deprecated.
#[doc(hidden)]
#[path = "deprecated.rs"]
pub mod __deprecated;

/// All internal machinery that is accessed by various gdext tools (e.g. proc macros).
#[doc(hidden)]
pub mod private;

/// Re-export logging macro.
#[doc(hidden)]
pub use godot_ffi::out;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Deprecated modules

#[deprecated = "Module has been split into `godot::classes`, `godot::global` and `godot::tools`."]
pub mod engine;

#[deprecated = "Print macros have been moved to `godot::global`."]
pub mod log {
    pub use crate::global::{
        godot_error, godot_print, godot_print_rich, godot_script_error, godot_warn,
    };
}
