/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// Whole module only available in Godot 4.2+.

mod connect_builder;
mod signal_object;
mod typed_signal;

pub(crate) mod variadic;
pub(crate) use connect_builder::*;
pub(crate) use signal_object::*;
pub(crate) use typed_signal::*;
pub(crate) use variadic::SignalReceiver;

// Used in `godot` crate.
pub mod re_export {
    pub use super::connect_builder::ConnectBuilder;
    pub use super::typed_signal::TypedSignal;
    pub use super::variadic::SignalReceiver;
}

// Used in `godot::private` module.
pub mod priv_re_export {
    pub use super::signal_object::{
        signal_collection_to_base, signal_collection_to_base_mut, UserSignalObject,
    };
}

// ParamTuple re-exported in crate::meta.
