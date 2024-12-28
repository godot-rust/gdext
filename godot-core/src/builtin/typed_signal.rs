/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::builtin::{Callable, Signal, Variant};
use crate::obj::{Gd, GodotClass};
use crate::{classes, meta};

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

// ----------------------------------------------------------------------------------------------------------------------------------------------

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

// ----------------------------------------------------------------------------------------------------------------------------------------------

pub struct TypedFunc<C, R, Ps> {
    godot_name: &'static str,
    _return_type: std::marker::PhantomData<R>,
    _param_types: std::marker::PhantomData<(C, Ps)>,
}

impl<C: GodotClass, R, Ps> TypedFunc<C, R, Ps> {
    #[doc(hidden)]
    pub fn from_godot_name(godot_name: &'static str) -> Self {
        Self {
            godot_name,
            _return_type: std::marker::PhantomData,
            _param_types: std::marker::PhantomData,
        }
    }

    pub fn with_object<T: GodotClass>(obj: &Gd<T>) {}

    pub fn godot_name(&self) -> &'static str {
        self.godot_name
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// `#[func]` reference that is readily callable.
///
/// Can be either a static function of a class, or a method which is bound to a concrete object.
pub struct Func<R, Ps> {
    godot_name: &'static str,
    bound_object: Option<Gd<classes::Object>>,
    _return_type: std::marker::PhantomData<R>,
    _param_types: std::marker::PhantomData<Ps>,
}

impl<R, Ps> Func<R, Ps> {
    pub fn to_callable(&self) -> Callable {
        // Instance method.
        if let Some(bound_object) = self.bound_object.as_ref() {
            return Callable::from_object_method(bound_object, self.godot_name);
        } else {
            return Callable::from_local_static();
        }

        // Static method.
    }
}
