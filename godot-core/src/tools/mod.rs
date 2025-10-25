/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Higher-level additions to the Godot engine API.
//!
//! Contains functionality that extends existing Godot classes and functions, to make them more versatile
//! or better integrated with Rust.

mod autoload;
mod gfile;
mod save_load;
mod translate;

pub use autoload::*;
pub use gfile::*;
pub use save_load::*;
pub use translate::*;

// ----------------------------------------------------------------------------------------------------------------------------------------------

pub(crate) fn cleanup() {
    clear_autoload_cache();
}
