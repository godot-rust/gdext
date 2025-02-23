/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// Maybe move this to builtin::functional module?

use crate::builtin::{Callable, GString, Variant};
use crate::classes::object::ConnectFlags;
use crate::obj::{bounds, Bounds, Gd, GodotClass, WithBaseField};
use crate::registry::functional::{AsFunc, ConnectBuilder, ParamTuple};
use crate::{classes, meta};
use std::borrow::Cow;
use std::marker::PhantomData;

#[doc(hidden)]
pub enum ObjectRef<'a, C: GodotClass> {
    /// Helpful for emit: reuse `&self` from within the `impl` block, goes through `base()` re-borrowing and thus allows re-entrant calls
    /// through Godot.
    Internal { obj_mut: &'a mut C },

    /// From outside, based on `Gd` pointer.
    External { gd: Gd<C> },
}

impl<C> ObjectRef<'_, C>
where
    C: WithBaseField,
{
    fn with_object_mut(&mut self, f: impl FnOnce(&mut classes::Object)) {
        match self {
            ObjectRef::Internal { obj_mut } => f(obj_mut.base_mut().upcast_object_mut()),
            ObjectRef::External { gd } => f(gd.upcast_object_mut()),
        }
    }

    fn to_owned(&self) -> Gd<C> {
        match self {
            ObjectRef::Internal { obj_mut } => WithBaseField::to_gd(*obj_mut),
            ObjectRef::External { gd } => gd.clone(),
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

pub struct TypedSignal<'c, C: GodotClass, Ps> {
    //signal: Signal,
    /// In Godot, valid signals (unlike funcs) are _always_ declared in a class and become part of each instance. So there's always an object.
    owner: ObjectRef<'c, C>,
    name: Cow<'static, str>,
    _signature: PhantomData<Ps>,
}

impl<'c, C: WithBaseField, Ps: ParamTuple> TypedSignal<'c, C, Ps> {
    #[doc(hidden)]
    pub fn new(owner: ObjectRef<'c, C>, name: &'static str) -> Self {
        Self {
            owner,
            name: Cow::Borrowed(name),
            _signature: PhantomData,
        }
    }

    pub(crate) fn receiver_object(&self) -> Gd<C> {
        self.owner.to_owned()
    }

    pub fn emit(&mut self, params: Ps) {
        let name = self.name.as_ref();

        self.owner.with_object_mut(|obj| {
            obj.emit_signal(name, &params.to_variant_array());
        });
    }

    /// Connect a non-member function (global function, associated function or closure).
    ///
    /// Example usages:
    /// ```ignore
    /// sig.connect(Self::static_func);
    /// sig.connect(global_func);
    /// sig.connect(|arg| { /* closure */ });
    /// ```
    ///
    /// To connect to a method of the own object `self`, use [`connect_self()`][Self::connect_self].
    pub fn connect<F>(&mut self, mut function: F)
    where
        F: AsFunc<(), Ps>,
    {
        let callable_name = std::any::type_name_of_val(&function);

        let godot_fn = move |variant_args: &[&Variant]| -> Result<Variant, ()> {
            let args = Ps::from_variant_array(variant_args);
            function.call((), args);

            Ok(Variant::nil())
        };

        self.inner_connect_local(callable_name, godot_fn);
    }

    /// Connect a method (member function) with `&mut self` as the first parameter.
    pub fn connect_self<F>(&mut self, mut function: F)
    where
        for<'c_rcv> F: AsFunc<&'c_rcv mut C, Ps>,
    {
        // When using sys::short_type_name() in the future, make sure global "func" and member "MyClass::func" are rendered as such.
        // PascalCase heuristic should then be good enough.
        let callable_name = std::any::type_name_of_val(&function);

        let object = self.owner.to_owned();
        let godot_fn = move |variant_args: &[&Variant]| -> Result<Variant, ()> {
            let args = Ps::from_variant_array(variant_args);

            // let mut function = function;
            // function.call(instance, args);
            let mut object = object.clone();

            // TODO: how to avoid another bind, when emitting directly from Rust?
            let mut instance = object.bind_mut();
            let instance = &mut *instance;
            function.call(instance, args);

            Ok(Variant::nil())
        };

        self.inner_connect_local(callable_name, godot_fn);
    }

    /// Connect a method (member function) with any `Gd<T>` (not `self`) as the first parameter.
    ///
    /// To connect to methods on the same object, use [`connect_self()`][Self::connect_self].
    pub fn connect_obj<F, OtherC>(&mut self, object: &Gd<OtherC>, mut function: F)
    where
        OtherC: GodotClass + Bounds<Declarer = bounds::DeclUser>,
        for<'c_rcv> F: AsFunc<&'c_rcv mut OtherC, Ps>,
    {
        let callable_name = std::any::type_name_of_val(&function);

        let mut object = object.clone();
        let godot_fn = move |variant_args: &[&Variant]| -> Result<Variant, ()> {
            let args = Ps::from_variant_array(variant_args);

            let mut instance = object.bind_mut();
            let instance = &mut *instance;
            function.call(instance, args);

            Ok(Variant::nil())
        };

        self.inner_connect_local(callable_name, godot_fn);
    }

    fn inner_connect_local<F>(&mut self, callable_name: impl meta::AsArg<GString>, godot_fn: F)
    where
        F: FnMut(&[&Variant]) -> Result<Variant, ()> + 'static,
    {
        let signal_name = self.name.as_ref();
        let callable = Callable::from_local_fn(callable_name, godot_fn);

        self.owner.with_object_mut(|obj| {
            obj.connect(signal_name, &callable);
        });
    }

    pub(super) fn connect_untyped(&mut self, callable: &Callable, flags: Option<ConnectFlags>) {
        use crate::obj::EngineEnum;

        let signal_name = self.name.as_ref();

        self.owner.with_object_mut(|obj| {
            let mut c = obj.connect_ex(signal_name, callable);
            if let Some(flags) = flags {
                c = c.flags(flags.ord() as u32);
            }
            c.done();
        });
    }

    pub fn connect_builder(&mut self) -> ConnectBuilder<'_, 'c, C, (), Ps, ()> {
        ConnectBuilder::new(self)
    }
}
