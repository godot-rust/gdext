/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

mod connect_builder;
mod connect_handle;
mod signal_object;
mod signal_receiver;
mod typed_signal;

pub(crate) use connect_builder::*;
pub(crate) use connect_handle::*;
pub(crate) use signal_object::*;
pub(crate) use typed_signal::*;

use crate::builtin::{GString, Variant};
use crate::meta;

// Used in `godot` crate.
pub mod re_export {
    pub use super::connect_builder::ConnectBuilder;
    pub use super::connect_handle::ConnectHandle;
    pub use super::signal_receiver::{IndirectSignalReceiver, SignalReceiver};
    pub use super::typed_signal::TypedSignal;
}

// Used in `godot::private` module.
pub mod priv_re_export {
    pub use super::signal_object::{
        signal_collection_to_base, signal_collection_to_base_mut, UserSignalObject,
    };
}

// ParamTuple re-exported in crate::meta.

// ----------------------------------------------------------------------------------------------------------------------------------------------

// Used by both `TypedSignal` and `ConnectBuilder`.
fn make_godot_fn<Ps, F>(mut input: F) -> impl FnMut(&[&Variant]) -> Result<Variant, ()>
where
    F: FnMut(Ps),
    Ps: meta::InParamTuple,
{
    move |variant_args: &[&Variant]| -> Result<Variant, ()> {
        let args = Ps::from_variant_array(variant_args);
        input(args);

        Ok(Variant::nil())
    }
}

// Used by both `TypedSignal` and `ConnectBuilder`.
fn make_callable_name<F>() -> GString {
    // When using sys::short_type_name() in the future, make sure global "func" and member "MyClass::func" are rendered as such.
    // PascalCase heuristic should then be good enough.

    std::any::type_name::<F>().into()
}
