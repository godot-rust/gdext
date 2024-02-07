/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Derive macros on types outside of classes.

mod data_models;
mod derive_export;
mod derive_from_godot;
mod derive_godot_convert;
mod derive_to_godot;
mod derive_var;

pub(crate) use derive_export::*;
pub(crate) use derive_from_godot::*;
pub(crate) use derive_godot_convert::*;
pub(crate) use derive_to_godot::*;
pub(crate) use derive_var::*;
