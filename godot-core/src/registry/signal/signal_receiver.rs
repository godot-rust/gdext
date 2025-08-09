/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Emulates variadic argument lists (via tuples), related to functions and signals.
// https://geo-ant.github.io/blog/2021/rust-traits-and-variadic-functions
//
// Could be generalized with R return type, and not special-casing `self`. But keep simple until actually needed.

use std::marker::PhantomData;

use crate::meta::{FromGodot, InParamTuple};
use crate::obj::{Gd, GodotClass};

/// Trait that is implemented for functions that can be connected to signals.
///
/// This is used in [`ConnectBuilder`](crate::registry::signal::connect_builder::ConnectBuilder).
/// There are three variations of the `C` (class instance) parameter:
/// - `()` for global and associated ("static") functions.
/// - `&C` for `&self` methods.
/// - `&mut C` for `&mut self` methods.
///
/// See also [Signals](https://godot-rust.github.io/book/register/signals.html) in the book.
///
/// # Usage as a bound
/// To write generic code that handles different functions and forwards them to [`TypedSignal`] and [`ConnectBuilder`] APIs,
/// you have to use `SignalReceiver` in combination with [`IndirectSignalReceiver`]. Consult the docs of the latter for elaboration.
///
/// [`TypedSignal`]: crate::registry::signal::TypedSignal
/// [`ConnectBuilder`]: crate::registry::signal::ConnectBuilder
///
/// # Hidden `impls` in this doc
/// To keep this documentation readable, we only document one variant of each `impl SignalReceiver`: arbitrarily the one with three parameters
/// `(P0, P1, P2)`. Keep this in mind when looking at a concrete signature.
pub trait SignalReceiver<C, Ps>: 'static {
    /// Invoke the receiver on the given instance (possibly `()`) with `params`.
    fn call(&mut self, maybe_instance: C, params: Ps);
}

// Next-gen trait solver should allow for type inference in closures, when function traits are involved, without using identity struct
// and other hacks. Since `IndirectSignalReceiver` is just a view, it should be drop-in replacement.
/// A special "identity struct" which enables type inference while specifying various closures for connections.
///
/// If you want to write generic code that can handle different functions and forward them to [`TypedSignal`] and [`ConnectBuilder`] APIs,
/// you have to use [`SignalReceiver`] in combination with this type. Please check its documentation for a detailed explanation.
///
/// [`TypedSignal`]: crate::registry::signal::TypedSignal
/// [`ConnectBuilder`]: crate::registry::signal::ConnectBuilder
///
/// # Background
///
/// Individual `connect*` methods on `TypedSignal` and `ConnectBuilder` use the `SignalReceiver` trait as a bound.  \
/// **In addition to that**, they have a rather complex bound involving [`IndirectSignalReceiver`]:
///
/// ```no_run
/// # use godot::register::{IndirectSignalReceiver, SignalReceiver};
/// # use godot_core::meta::InParamTuple;
/// # use godot_core::obj::WithSignals;
/// # struct TypedSignal<'c, C: WithSignals, Ps> { _phantom: std::marker::PhantomData<&'c (C, Ps)> }
/// impl<C: WithSignals, Ps: InParamTuple + 'static> TypedSignal<'_, C, Ps> {
///     pub fn connect<F>(&self, mut function: F)
///     where
///         for<'c_rcv> F: SignalReceiver<(), Ps> + 'static,
///         for<'c_rcv> IndirectSignalReceiver<'c_rcv, (), Ps, F>: From<&'c_rcv mut F>,
///     { /* ... */ }
/// }
/// ```
///
/// This second bound is necessary because rustc cannot infer parameter types in closures, when dealing with `Fn/FnMut/FnOnce` traits abstracted
/// behind another trait (in our case [`SignalReceiver`]). Without inference, it wouldn't be reliably possible to pass `|this, arg| { ... }`
/// closures and would require the more verbose `|this: &mut MyClass, arg: GString| { ... }` syntax.
///
/// To make type inference work in such cases, we need to specify concrete type, such as `FnMut(...) -> R` or
/// `<&mut F as Into<IndirectSignalReceiver<'_, MyClass, (Param0, Param1), Func>>>`. The ~~dirty hack~~ clever trick used here is to "smuggle" a
/// concrete `Fn*` trait through `IndirectSignalReceiver`. This forces rustc to resolve the concrete `Fn*` type.
///
/// Prior designs included declarative macros to generate all `connect*` functions with direct `Fn*` bounds (not through `SignalReceiver`).
/// This works well, too, but prevents users from extending the functionality through generic programming -- they'd need to use macros, too.
///
/// # Usage within `connect*` style methods
/// When using the trait bounds as described above, you can access the actual function in the following way:
///
/// ```no_run
/// # use godot::register::{IndirectSignalReceiver, SignalReceiver};
/// # let mut function = || {};
/// # let args = ();
/// IndirectSignalReceiver::from(&mut function)
///     .function()
///     .call((), args);
/// ```
///
/// # Hidden `impls` in this doc
/// To keep this documentation readable, we only document one variant of each `impl From<F>`: arbitrarily the one with three parameters
/// `(P0, P1, P2)`. Keep this in mind when looking at a concrete signature.
///
/// # Further reading
/// - [rustc issue #63702](https://github.com/rust-lang/rust/issues/63702)
/// - [identity function trick](https://users.rust-lang.org/t/type-inference-in-closures/78399/3)
/// - [discussion about type-inference limits](https://users.rust-lang.org/t/what-are-the-limits-of-type-inference-in-closures/31519)
/// - [rustc comments around closure type-check](https://github.com/rust-lang/rust/blob/5ad7454f7503b6af2800bf4a7c875962cb03f913/compiler/rustc_hir_typeck/src/fn_ctxt/checks.rs#L306-L317)
pub struct IndirectSignalReceiver<'view, C, Ps, F>
where
    Ps: InParamTuple,
    F: SignalReceiver<C, Ps> + 'static,
{
    inner: &'view mut F,
    _phantoms: PhantomData<(C, Ps)>,
}

impl<'view, C, Ps, F> IndirectSignalReceiver<'view, C, Ps, F>
where
    Ps: InParamTuple,
    F: SignalReceiver<C, Ps> + 'static,
{
    /// Retrieves inner `&mut F` function ready to be used as [`SignalReceiver`].
    pub fn function(&'view mut self) -> &'view mut F {
        self.inner
    }

    /// Creates a new `IndirectSignalReceiver` from a mutable reference to a function.
    fn new(inner: &'view mut F) -> Self {
        Self {
            inner,
            _phantoms: PhantomData,
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Generated impls

macro_rules! impl_signal_recipient {
    ($( #[$attr:meta] )? $( $args:ident : $Ps:ident ),*) => {
        // --------------------------------------------------------------------------------------------------------------------------------------
        // SignalReceiver

        // SignalReceiver: Global and associated functions.
        $( #[$attr] )?
        impl<F, R, $($Ps: std::fmt::Debug + FromGodot + 'static),*> SignalReceiver<(), ( $($Ps,)* )> for F
            where F: FnMut( $($Ps,)* ) -> R + 'static
        {
            fn call(&mut self, _no_instance: (), ($($args,)*): ( $($Ps,)* )) {
                self($($args,)*);
            }
        }

        // SignalReceiver: Methods with mutable receiver - &mut self.
        $( #[$attr] )?
        impl<F, R, C, $($Ps: std::fmt::Debug + FromGodot + 'static),*> SignalReceiver<&mut C, ( $($Ps,)* )> for F
            where F: FnMut( &mut C, $($Ps,)* ) -> R + 'static
        {
            fn call(&mut self, instance: &mut C, ($($args,)*): ( $($Ps,)* )) {
                self(instance, $($args,)*);
            }
        }

        // Methods with immutable receiver - &self. Disabled until needed.
        /*
        $( #[$attr] )?
        impl<F, R, C, $($Ps: std::fmt::Debug + FromGodot + 'static),*> SignalReceiver<&C, ( $($Ps,)* )> for F
            where F: FnMut( &C, $($Ps,)* ) -> R + 'static
        {
            fn call(&mut self, instance: &C, ($($args,)*): ( $($Ps,)* )) {
                self(instance, $($args,)*);
            }
        }
        */

        // Methods with Gd receiver - Gd<Self>.
        $( #[$attr] )?
        impl<F, R, C, $($Ps: std::fmt::Debug + FromGodot + 'static),*> SignalReceiver<Gd<C>, ( $($Ps,)* )> for F
            where F: FnMut( Gd<C>, $($Ps,)* ) -> R + 'static, C: GodotClass
        {
            fn call(&mut self, instance: Gd<C>, ($($args,)*): ( $($Ps,)* )) {
                self(instance, $($args,)*);
            }
        }

        // --------------------------------------------------------------------------------------------------------------------------------------
        // FnMut -> IndirectSignalReceiver

        // From: Global and associated functions.
        $( #[$attr] )?
        impl<'c_view, F, R, $($Ps: std::fmt::Debug + FromGodot + 'static),*>
            From<&'c_view mut F> for IndirectSignalReceiver<'c_view, (), ($($Ps,)*), F>
            where F: FnMut( $($Ps,)* ) -> R + 'static
        {
            fn from(value: &'c_view mut F) -> Self {
                IndirectSignalReceiver::new(value)
            }
        }

        // From: Methods with mutable receiver - &mut self.
        $( #[$attr] )?
        impl<'c_view, F, R, C, $($Ps: std::fmt::Debug + FromGodot + 'static),*>
            From<&'c_view mut F> for IndirectSignalReceiver<'c_view, &mut C, ($($Ps,)*), F>
            where F: FnMut( &mut C, $($Ps,)* ) -> R + 'static
        {
            fn from(value: &'c_view mut F) -> Self {
                IndirectSignalReceiver::new(value)
            }
        }

        // From: Methods with immutable receiver - &self. Disabled until needed.
        /*
        $( #[$attr] )?
        impl<'c_view, F, R, C, $($Ps: std::fmt::Debug + FromGodot + 'static),*>
            From<&'c_view mut F> for IndirectSignalReceiver<'c_view, &C, ($($Ps,)*), F>
            where F: FnMut( &C, $($Ps,)* ) -> R + 'static
        {
            fn from(value: &'c_view mut F) -> Self {
                IndirectSignalReceiver::new(value)
            }
        }
        */

        // From: Methods with Gd receiver - Gd<Self>.
        $( #[$attr] )?
        impl<'c_view, F, R, C, $($Ps: std::fmt::Debug + FromGodot + 'static),*>
            From<&'c_view mut F> for IndirectSignalReceiver<'c_view, Gd<C>, ($($Ps,)*), F>
            where F: FnMut( Gd<C>, $($Ps,)* ) -> R + 'static, C: GodotClass
        {
            fn from(value: &'c_view mut F) -> Self {
                IndirectSignalReceiver::new(value)
            }
        }
    };
}

impl_signal_recipient!(#[doc(hidden)] );
impl_signal_recipient!(#[doc(hidden)] arg0: P0);
impl_signal_recipient!(#[doc(hidden)] arg0: P0, arg1: P1);
impl_signal_recipient!(               arg0: P0, arg1: P1, arg2: P2);
impl_signal_recipient!(#[doc(hidden)] arg0: P0, arg1: P1, arg2: P2, arg3: P3);
impl_signal_recipient!(#[doc(hidden)] arg0: P0, arg1: P1, arg2: P2, arg3: P3, arg4: P4);
impl_signal_recipient!(#[doc(hidden)] arg0: P0, arg1: P1, arg2: P2, arg3: P3, arg4: P4, arg5: P5);
impl_signal_recipient!(#[doc(hidden)] arg0: P0, arg1: P1, arg2: P2, arg3: P3, arg4: P4, arg5: P5, arg6: P6);
impl_signal_recipient!(#[doc(hidden)] arg0: P0, arg1: P1, arg2: P2, arg3: P3, arg4: P4, arg5: P5, arg6: P6, arg7: P7);
impl_signal_recipient!(#[doc(hidden)] arg0: P0, arg1: P1, arg2: P2, arg3: P3, arg4: P4, arg5: P5, arg6: P6, arg7: P7, arg8: P8);
impl_signal_recipient!(#[doc(hidden)] arg0: P0, arg1: P1, arg2: P2, arg3: P3, arg4: P4, arg5: P5, arg6: P6, arg7: P7, arg8: P8, arg9: P9);
