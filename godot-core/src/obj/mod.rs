/*
 * Copyright (c) godot-rust; Bromeon and contributors.
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
mod onready;
mod raw;
mod traits;

pub(crate) mod rtti;

pub use base::*;
pub use gd::*;
pub use guards::{BaseMut, BaseRef, GdMut, GdRef};
pub use instance_id::*;
pub use onready::*;
pub use raw::*;
pub use traits::*;

pub mod bounds;
pub mod script;
pub use bounds::private::Bounds;

// Do not re-export rtti here.

type GdDerefTarget<T> = <<T as Bounds>::Declarer as bounds::Declarer>::DerefTarget<T>;
