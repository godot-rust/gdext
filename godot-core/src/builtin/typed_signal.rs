/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::builtin::{Callable, Signal, Variant};
use crate::meta;

pub trait ParamTuple {
    fn to_variant_array(&self) -> Vec<Variant>;
}

impl ParamTuple for () {
    fn to_variant_array(&self) -> Vec<Variant> {
        Vec::new()
    }
}

impl<T> ParamTuple for (T,)
where
    T: meta::ToGodot,
{
    fn to_variant_array(&self) -> Vec<Variant> {
        vec![self.0.to_variant()]
    }
}

pub struct TypedSignal<Ps> {
    signal: Signal,
    _signature: std::marker::PhantomData<Ps>,
}

impl<Ps: ParamTuple> TypedSignal<Ps> {
    pub(crate) fn from_untyped(signal: Signal) -> Self {
        Self {
            signal,
            _signature: std::marker::PhantomData,
        }
    }

    pub fn emit(&self, params: Ps) {
        self.signal.emit(&params.to_variant_array());
    }

    pub fn connect_untyped(&mut self, callable: &Callable, flags: i64) {
        self.signal.connect(callable, flags);
    }

    pub fn to_untyped(&self) -> Signal {
        self.signal.clone()
    }
}
