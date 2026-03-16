/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Custom error types.

mod call_error;
mod call_error_type;
mod convert_error;
mod error_to_godot;
mod io_error;
mod rpc_error;
mod string_error;

pub mod strat;

pub use call_error::*;
pub use call_error_type::*;
pub use convert_error::*;
pub use error_to_godot::*;
pub use io_error::*;
pub use rpc_error::*;
pub use string_error::*;

pub use crate::func_bail;
