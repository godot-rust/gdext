#![cfg_attr(published_docs, feature(doc_cfg))]
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
#[cfg(all(since_api = "4.3", feature = "register-docs"))] #[cfg_attr(published_docs, doc(cfg(all(since_api = "4.3", feature = "register-docs"))))]
pub mod docs;
#[doc(hidden)]
pub mod possibly_docs {
    #[cfg(all(since_api = "4.3", feature = "register-docs"))] #[cfg_attr(published_docs, doc(cfg(all(since_api = "4.3", feature = "register-docs"))))]
    pub use crate::docs::*;
}
pub mod global;
pub mod init;
pub mod meta;
pub mod obj;
pub mod registry;
pub mod task;
pub mod tools;

mod storage;
pub use godot_ffi as sys;

pub use crate::private::{get_gdext_panic_context, set_gdext_hook};

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Validations (see also godot/lib.rs)

#[cfg(all(feature = "register-docs", before_api = "4.3"))] #[cfg_attr(published_docs, doc(cfg(all(feature = "register-docs", before_api = "4.3"))))]
compile_error!("Generating editor docs for Rust symbols requires at least Godot 4.3.");

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Generated code

// Output of generated code. Mimics the file structure, symbols are re-exported.
#[rustfmt::skip]
#[allow(unused_imports, dead_code, non_upper_case_globals, non_snake_case)]
#[allow(clippy::too_many_arguments, clippy::let_and_return, clippy::new_ret_no_self)]
#[allow(clippy::let_unit_value)] // let args = ();
#[allow(clippy::wrong_self_convention)] // to_string() is const
#[allow(clippy::upper_case_acronyms)] // TODO remove this line once we transform names
#[allow(clippy::needless_lifetimes)]  // the following explicit lifetimes could be elided: 'a
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
