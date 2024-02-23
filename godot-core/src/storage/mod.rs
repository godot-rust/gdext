/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

mod instance_storage;
#[cfg_attr(not(feature = "experimental-threads"), allow(dead_code))]
mod multi_threaded;
#[cfg_attr(feature = "experimental-threads", allow(dead_code))]
mod single_threaded;

pub use instance_storage::*;
