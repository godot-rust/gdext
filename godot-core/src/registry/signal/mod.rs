/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

mod connect_builder;
mod signal_object;
mod typed_signal;
pub(crate) mod variadic;

pub use connect_builder::*;
pub use signal_object::{SignalObject, UserSignalObject};
pub use typed_signal::*;
pub use variadic::SignalReceiver;

// ParamTuple re-exported in crate::meta.
