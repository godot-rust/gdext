/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

mod gfile;
mod io_error;
mod resources;

pub use gfile::GFile;
pub use io_error::*;
pub use resources::{load, save, try_load, try_save};
