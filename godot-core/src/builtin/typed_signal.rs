/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// Maybe move this to builtin::functional module?

use crate::builtin::{Callable, Signal, Variant};
use crate::obj::Gd;
use crate::{classes, meta, sys};
use std::borrow::Cow;
use std::fmt;

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
/* Previous impl based on assumption, Signal would be used. Could maybe be combined within an enum.

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
}*/

// ----------------------------------------------------------------------------------------------------------------------------------------------

/*
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
*/

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Type-safe `#[func]` reference that is readily callable.
///
/// Can be either a static function of a class, or a method which is bound to a concrete object.
///
/// This can be seen as a more type-safe variant of Godot's `Callable`, which can carry intermediate information about function signatures (e.g.
/// when connecting signals).
pub struct Func<R, Ps> {
    godot_function_name: &'static str,
    callable_kind: CallableKind,
    _return_type: std::marker::PhantomData<R>,
    _param_types: std::marker::PhantomData<Ps>,
}

enum CallableKind {
    StaticFunction {
        // Maybe class name can be moved out (and also be useful for methods), e.g. Debug impl or so.
        class_godot_name: Cow<'static, str>,
    },
    Method {
        bound_object: Gd<classes::Object>,
    },
}

impl<R, Ps> Func<R, Ps> {
    #[doc(hidden)]
    pub fn from_instance_method(
        bound_object: Gd<classes::Object>,
        method_godot_name: &'static str,
    ) -> Self {
        Self {
            godot_function_name: method_godot_name,
            callable_kind: CallableKind::Method { bound_object },
            _return_type: std::marker::PhantomData,
            _param_types: std::marker::PhantomData,
        }
    }

    #[doc(hidden)]
    pub fn from_static_function(
        class_godot_name: Cow<'static, str>,
        method_godot_name: &'static str,
    ) -> Self {
        Self {
            godot_function_name: method_godot_name,
            callable_kind: CallableKind::StaticFunction { class_godot_name },
            _return_type: std::marker::PhantomData,
            _param_types: std::marker::PhantomData,
        }
    }

    pub fn to_callable(&self) -> Callable {
        match &self.callable_kind {
            CallableKind::StaticFunction { class_godot_name } => {
                let class_name = class_godot_name.as_ref();
                Callable::from_local_static(class_name, self.godot_function_name)
            }
            CallableKind::Method { bound_object } => {
                Callable::from_object_method(bound_object, self.godot_function_name)
            }
        }
    }
}

impl<R, Ps> fmt::Debug for Func<R, Ps> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let r = sys::short_type_name::<R>();
        let ps = sys::short_type_name::<Ps>();

        let (obj_or_class, is_static);
        match &self.callable_kind {
            CallableKind::StaticFunction { class_godot_name } => {
                obj_or_class = class_godot_name.to_string();
                is_static = "; static";
            }
            CallableKind::Method { bound_object } => {
                obj_or_class = format!("{bound_object:?}");
                is_static = "";
            }
        };

        let function = self.godot_function_name;
        write!(f, "Func({obj_or_class}.{function}{is_static}; {ps} -> {r})")
    }
}
