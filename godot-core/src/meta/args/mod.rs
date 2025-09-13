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

pub use as_arg::{owned_into_arg, ref_to_arg, ArgPassing, AsArg, ByObject, ByRef, ByValue, ToArg};
// ----------------------------------------------------------------------------------------------------------------------------------------------
// Internal APIs

// Solely public for itest/convert_test.rs.
#[cfg(feature = "trace")]
#[doc(hidden)]
pub use cow_arg::CowArg;
#[cfg(not(feature = "trace"))]
pub(crate) use cow_arg::CowArg;
#[allow(unused)] // TODO(v0.4): replace contents with newer changes
pub use object_arg::ObjectArg;
pub use ref_arg::RefArg;

// #[doc(hidden)]
// pub use cow_arg::*;
//
// #[doc(hidden)]
// pub use ref_arg::*;
