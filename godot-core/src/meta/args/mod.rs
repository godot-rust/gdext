/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

mod as_arg;
mod cow_arg;
mod object_arg;
mod ref_arg;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Public APIs

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Internal APIs

// Solely public for itest/convert_test.rs.
pub(crate) use as_arg::NullArg;
pub use as_arg::{
    owned_into_arg, ref_to_arg, ArgPassing, AsArg, ByObject, ByOption, ByRef, ByValue, ToArg,
};
#[cfg(not(feature = "trace"))]
pub(crate) use cow_arg::{CowArg, FfiArg};
// Integration test only.
#[cfg(feature = "trace")]
#[doc(hidden)]
pub use cow_arg::{CowArg, FfiArg};
pub use object_arg::ObjectArg;
pub(crate) use ref_arg::RefArg;
