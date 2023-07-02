/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Types and traits related to objects.
//!
//! The most important symbols in this module are:
//! * [`GodotClass`], which is implemented for every class that Godot can work with (either engine- or user-provided).
//! * [`Gd`], a smart pointer that manages instances of Godot classes.

mod base;
mod gd;
mod guards;
mod instance_id;
mod traits;

pub use base::*;
pub use gd::*;
pub use guards::*;
pub use instance_id::*;
pub use traits::*;
