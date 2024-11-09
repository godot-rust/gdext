/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

mod as_arg;
mod cow_arg;
mod ref_arg;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Public APIs

pub use as_arg::*;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Internal APIs

#[doc(hidden)]
pub use cow_arg::*;

#[doc(hidden)]
pub use ref_arg::*;
