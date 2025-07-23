/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use super::{make_callable_name, make_godot_fn, ConnectBuilder, ConnectHandle, SignalObject};
use crate::builtin::{Callable, Variant};
use crate::classes::object::ConnectFlags;
use crate::meta;
use crate::meta::{InParamTuple, UniformObjectDeref};
use crate::obj::{Gd, GodotClass, WithBaseField, WithSignals};
use crate::registry::signal::signal_receiver::{IndirectSignalReceiver, SignalReceiver};
use std::borrow::Cow;
use std::marker::PhantomData;
use std::ops::DerefMut;

// TODO(v0.4): find more general name for trait.
/// Object part of the signal receiver (handler).
///
/// Functionality overlaps partly with [`meta::AsObjectArg`] and [`meta::AsArg<ObjectArg>`]. Can however not directly be replaced
/// with `AsObjectArg`, since that allows nullability and doesn't require `&mut T`. Maybe there's a way to reuse them though.
pub trait ToSignalObj<C: GodotClass> {
    fn to_signal_obj(&self) -> Gd<C>;
}

impl<C: GodotClass> ToSignalObj<C> for Gd<C> {
    fn to_signal_obj(&self) -> Gd<C> {
        self.clone()
    }
}

impl<C: WithBaseField> ToSignalObj<C> for C {
    fn to_signal_obj(&self) -> Gd<C> {
        WithBaseField::to_gd(self)
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Type-safe version of a Godot signal.
///
/// Short-lived type, only valid in the scope of its surrounding object type `C`, for lifetime `'c`. The generic argument `Ps` represents
/// the parameters of the signal, thus ensuring the type safety.
///
/// See the [Signals](https://godot-rust.github.io/book/register/signals.html) chapter in the book for a general introduction and examples.
///
/// # Listing signals of a class
/// The [`WithSignals::SignalCollection`] struct stores multiple signals with distinct, code-generated types, but they all implement
/// `Deref` and `DerefMut` to `TypedSignal`. This allows you to either use the concrete APIs of the generated types, or the more generic
/// ones of `TypedSignal`.
///
/// You can access the signal collection of a class via [`self.signals()`][crate::obj::WithUserSignals::signals] or
/// [`Gd::signals()`][Gd::signals].
///
/// # Connecting a signal to a receiver
/// Receiver functions are functions that are called when a signal is emitted. You can connect a signal in many different ways:
/// - [`connect()`][Self::connect]: Connect a global/associated function or a closure.
/// - [`connect_self()`][Self::connect_self]: Connect a method or closure that runs on the signal emitter.
/// - [`connect_other()`][Self::connect_other]: Connect a method or closure that runs on a separate object.
/// - [`builder()`][Self::builder] for more complex setups (such as choosing [`ConnectFlags`] or making thread-safe connections).
///
/// # Emitting a signal
/// Code-generated signal types provide a method `emit(...)`, which adopts the names and types of the `#[signal]` parameter list.
/// In most cases, that's the method you are looking for.
///
/// For generic use, you can also use [`emit_tuple()`][Self::emit_tuple], which does not provide parameter names.
///
/// # Generic programming and code reuse
/// If you want to build higher-level abstractions that operate on `TypedSignal`, you will need the [`SignalReceiver`] trait.
pub struct TypedSignal<'c, C: WithSignals, Ps> {
    /// In Godot, valid signals (unlike funcs) are _always_ declared in a class and become part of each instance. So there's always an object.
    object: C::__SignalObj<'c>,
    name: Cow<'static, str>,
    _signature: PhantomData<Ps>,
}

impl<'c, C: WithSignals, Ps: meta::ParamTuple> TypedSignal<'c, C, Ps> {
    #[doc(hidden)]
    pub fn extract(
        obj: &mut Option<C::__SignalObj<'c>>,
        signal_name: &'static str,
    ) -> TypedSignal<'c, C, Ps> {
        let obj = obj.take().unwrap_or_else(|| {
            panic!(
                "signals().{signal_name}() call failed; signals() allows only one signal configuration at a time \n\
                see https://godot-rust.github.io/book/register/signals.html#admonition-one-signal-at-a-time"
            )
        });

        Self::new(obj, signal_name)
    }

    // Currently only invoked from godot-core classes, or from UserSignalObject::into_typed_signal.
    // When making public, make also #[doc(hidden)].
    fn new(object: C::__SignalObj<'c>, name: &'static str) -> Self {
        Self {
            object,
            name: Cow::Borrowed(name),
            _signature: PhantomData,
        }
    }

    pub(crate) fn receiver_object(&self) -> Gd<C> {
        let object = self.object.to_owned_object();

        // Potential optimization: downcast could use a new private Gd::unchecked_cast().
        // try_cast().unwrap_unchecked() won't be that efficient due to internal code path.
        object.cast()
    }

    /// Fully customizable connection setup.
    ///
    /// The returned builder provides several methods to configure how to connect the signal. It needs to be finalized with a call
    /// to any of the builder's `connect_*` methods.
    pub fn builder<'ts>(&'ts self) -> ConnectBuilder<'ts, 'c, C, Ps> {
        ConnectBuilder::new(self)
    }

    /// Emit the signal with the given parameters.
    ///
    /// This is intended for generic use. Typically, you'll want to use the more specific `emit()` method of the code-generated signal
    /// type, which also has named parameters.
    pub fn emit_tuple(&mut self, args: Ps)
    where
        Ps: meta::OutParamTuple,
    {
        let name = self.name.as_ref();

        self.object.with_object_mut(|obj| {
            obj.emit_signal(name, &args.to_variant_array());
        });
    }

    /// Directly connect a Rust callable `godot_fn`, with a name based on `F` bound to given object.
    ///
    /// Signal will be automatically disconnected by Godot after bound object will be freed.
    ///
    /// This exists as a shorthand for the connect methods on [`TypedSignal`] and avoids the generic instantiation of the full-blown
    /// type state builder for simple + common connections, thus hopefully being a tiny bit lighter on compile times.
    fn inner_connect_godot_fn<F>(
        &self,
        godot_fn: impl FnMut(&[&Variant]) -> Result<Variant, ()> + 'static,
        bound: &Gd<impl GodotClass>,
    ) -> ConnectHandle {
        let callable_name = make_callable_name::<F>();
        let callable = bound.linked_callable(&callable_name, godot_fn);
        self.inner_connect_untyped(callable, None)
    }

    /// Connect an untyped callable, with optional flags.
    ///
    /// Used by [`inner_connect_godot_fn`] and `ConnectBuilder::connect_sync`.
    pub(super) fn inner_connect_untyped(
        &self,
        callable: Callable,
        flags: Option<ConnectFlags>,
    ) -> ConnectHandle {
        use crate::obj::EngineBitfield;

        let signal_name = self.name.as_ref();

        let mut owned_object = self.object.to_owned_object();
        owned_object.with_object_mut(|obj| {
            let mut c = obj.connect_ex(signal_name, &callable);
            if let Some(flags) = flags {
                c = c.flags(flags.ord() as u32);
            }
            c.done();
        });

        ConnectHandle::new(owned_object, self.name.clone(), callable)
    }

    pub(crate) fn to_untyped(&self) -> crate::builtin::Signal {
        crate::builtin::Signal::from_object_signal(&self.receiver_object(), &*self.name)
    }
}

impl<C: WithSignals, Ps: InParamTuple + 'static> TypedSignal<'_, C, Ps> {
    /// Connect a non-member function (global function, associated function or closure).
    ///
    /// Example usages:
    /// ```ignore
    /// sig.connect(Self::static_func);
    /// sig.connect(global_func);
    /// sig.connect(|arg| { /* closure */ });
    /// ```
    ///
    /// - To connect to a method on the object that owns this signal, use [`connect_self()`][Self::connect_self].
    /// - If you need [`connect flags`](ConnectFlags) or cross-thread signals, use [`builder()`][Self::builder].
    pub fn connect<F>(&self, mut function: F) -> ConnectHandle
    where
        for<'c_rcv> F: SignalReceiver<(), Ps> + 'static,
        for<'c_rcv> IndirectSignalReceiver<'c_rcv, (), Ps, F>: From<&'c_rcv mut F>,
    {
        let godot_fn = make_godot_fn(move |args| {
            IndirectSignalReceiver::from(&mut function)
                .function()
                .call((), args);
        });

        self.inner_connect_godot_fn::<F>(godot_fn, &self.receiver_object())
    }

    /// Connect a method (member function) with `&mut self` as the first parameter.
    ///
    /// - To connect to methods on other objects, use [`connect_other()`][Self::connect_other].
    /// - If you need [`connect flags`](ConnectFlags) or cross-thread signals, use [`builder()`][Self::builder].
    pub fn connect_self<F, Declarer>(&self, mut function: F) -> ConnectHandle
    where
        for<'c_rcv> F: SignalReceiver<&'c_rcv mut C, Ps> + 'static,
        for<'c_rcv> IndirectSignalReceiver<'c_rcv, &'c_rcv mut C, Ps, F>: From<&'c_rcv mut F>,
        C: UniformObjectDeref<Declarer>,
    {
        let mut gd = self.receiver_object();
        let godot_fn = make_godot_fn(move |args| {
            let mut target = C::object_as_mut(&mut gd);
            let target_mut = target.deref_mut();
            IndirectSignalReceiver::from(&mut function)
                .function()
                .call(target_mut, args);
        });

        self.inner_connect_godot_fn::<F>(godot_fn, &self.receiver_object())
    }

    /// Connect a method (member function) with any `&mut OtherC` as the first parameter, where
    /// `OtherC`: [`GodotClass`](GodotClass) (both user and engine classes are accepted).
    ///
    /// The parameter `object` can be of 2 different "categories":
    /// - Any `&Gd<OtherC>` (e.g.: `&Gd<Node>`, `&Gd<CustomUserClass>`).
    /// - `&OtherC`, as long as `OtherC` is a user class that contains a `base` field (it implements the
    ///   [`WithBaseField`](WithBaseField) trait).
    ///
    /// ---
    ///
    /// - To connect to methods on the object that owns this signal, use [`connect_self()`][Self::connect_self].
    /// - If you need [`connect flags`](ConnectFlags) or cross-thread signals, use [`builder()`][Self::builder].
    pub fn connect_other<F, OtherC, Declarer>(
        &self,
        object: &impl ToSignalObj<OtherC>,
        mut method: F,
    ) -> ConnectHandle
    where
        OtherC: UniformObjectDeref<Declarer>,
        for<'c_rcv> F: SignalReceiver<&'c_rcv mut OtherC, Ps> + 'static,
        for<'c_rcv> IndirectSignalReceiver<'c_rcv, &'c_rcv mut OtherC, Ps, F>: From<&'c_rcv mut F>,
    {
        let mut gd = object.to_signal_obj();

        let godot_fn = make_godot_fn(move |args| {
            let mut target = OtherC::object_as_mut(&mut gd);
            let target_mut = target.deref_mut();
            IndirectSignalReceiver::from(&mut method)
                .function()
                .call(target_mut, args);
        });

        self.inner_connect_godot_fn::<F>(godot_fn, &object.to_signal_obj())
    }
}
