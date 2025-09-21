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
mod casts;
mod dyn_gd;
mod gd;
mod guards;
mod instance_id;
mod on_editor;
mod on_ready;
mod passive_gd;
mod raw_gd;
mod traits;

pub(crate) mod rtti;

pub use base::*;
pub use dyn_gd::DynGd;
pub use gd::*;
pub use guards::{BaseMut, BaseRef, DynGdMut, DynGdRef, GdMut, GdRef};
pub use instance_id::*;
pub use on_editor::*;
pub use on_ready::*;
pub(crate) use passive_gd::PassiveGd;
pub use raw_gd::*;
pub use traits::*;

pub mod bounds;
pub mod script;
pub use bounds::private::Bounds;

// Do not re-export rtti here.

/// Resolves the type to which a `Gd<T>` dereferences.
///
/// This type alias abstracts over the two `Declarer` options for Godot objects:
/// - [`bounds::DeclEngine`]: for all engine-provided classes, `DerefTarget<T>` is `T`.
/// - [`bounds::DeclUser`]: for Rust-defined user classes, `DerefTarget<T>` is `T::Base`.
type GdDerefTarget<T> = <<T as Bounds>::Declarer as bounds::Declarer>::DerefTarget<T>;
