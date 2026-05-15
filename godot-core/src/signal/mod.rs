/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Type-safe signals: connecting, emitting, and handling.

mod connect_builder;
mod connect_handle;
mod signal_connections_registry;
mod signal_object;
mod signal_receiver;
mod typed_signal;

use std::borrow::Cow;

// Public API -- re-exported as `godot::signal`.
pub use connect_builder::ConnectBuilder;
pub use connect_handle::ConnectHandle;
pub use signal_receiver::{IndirectSignalReceiver, SignalReceiver};
pub use typed_signal::TypedSignal;

// Bridge for `godot::private` (proc-macro internals).
#[doc(hidden)]
pub mod priv_re_export {
    pub use super::signal_object::{
        UserSignalObject, signal_collection_to_base, signal_collection_to_base_mut,
    };
}

// Crate-internal items used outside this module.
pub(crate) use signal_connections_registry::prune_stored_signal_connections;
pub(crate) use signal_object::SignalObject;

use crate::builtin::{CowStr, Variant};
use crate::meta;

// ----------------------------------------------------------------------------------------------------------------------------------------------

// Used by both `TypedSignal` and `ConnectBuilder`.
fn make_godot_fn<Ps, F>(mut input: F) -> impl FnMut(&[&Variant]) -> Variant
where
    F: FnMut(Ps),
    Ps: meta::InParamTuple,
{
    move |variant_args: &[&Variant]| {
        let args = Ps::from_variant_array(variant_args);
        input(args);
        Variant::nil()
    }
}

// Used by both `TypedSignal` and `ConnectBuilder`.
fn make_callable_name<F>() -> CowStr {
    // When using sys::short_type_name() in the future, make sure global "func" and member "MyClass::func" are rendered as such.
    // PascalCase heuristic should then be good enough.

    Cow::Borrowed(std::any::type_name::<F>())
}
