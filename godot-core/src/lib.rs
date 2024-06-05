/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! # Internal crate of [**godot-rust**](https://godot-rust.github.io)
//!
//! Do not depend on this crate directly, instead use the `godot` crate.
//! No SemVer or other guarantees are provided.

// Note that a lot of those are public, but the godot crate still has the final say on what it wants to re-export.
// Doing fine-grained visibility restrictions on every level is a useless maintenance chore.
pub mod builder;
pub mod builtin;
pub mod classes;
pub mod global;
pub mod init;
pub mod meta;
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
#[doc(hidden)] // No longer advertise in API docs.
pub mod engine;

#[deprecated = "Print macros have been moved to `godot::global`."]
#[doc(hidden)] // No longer advertise in API docs.
pub mod log {
    pub use crate::global::{
        godot_error, godot_print, godot_print_rich, godot_script_error, godot_warn,
    };
}
