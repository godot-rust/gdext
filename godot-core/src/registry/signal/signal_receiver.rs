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

use crate::meta::FromGodot;
use crate::meta::InParamTuple;
use crate::obj::{Gd, GodotClass};
use std::marker::PhantomData;

/// Trait that is implemented for functions that can be connected to signals.
///
/// This is used in [`ConnectBuilder`](crate::registry::signal::connect_builder::ConnectBuilder).
/// There are three variations of the `C` (class instance) parameter:
/// - `()` for global and associated ("static") functions.
/// - `&C` for `&self` methods.
/// - `&mut C` for `&mut self` methods.
///
/// See also [Signals](https://godot-rust.github.io/book/register/signals.html) in the book.
pub trait SignalReceiver<C, Ps>: 'static {
    /// Invoke the receiver on the given instance (possibly `()`) with `params`.
    fn call(&mut self, maybe_instance: C, params: Ps);
}

// Next-gen trait solver should allow for type inference in closures, when function traits are involved, without using identity struct
// and other hacks. Since `IndirectSignalReceiver` is just a view, it should be drop-in replacement.
/// A special "identity struct" which allows to use type inference while specifying various closures for connections.
///
/// rustc can't infer types in closures when dealing with `Fn/FnMut/FnOnce` traits abstracted behind another trait (in our case
/// [`SignalReceiver`]). To make type inference work in such cases, we need to specify concrete type – such as `FnMut(...) -> R`
/// or `<&mut F as Into<IndirectSignalReceiver<'_, Instance, Params, Func>>>`.
///
/// In other words, `IndirectSignalReceiver` allows us to "smuggle" in a `Fn*` trait, forcing rustc to deal with type inference while resolving
/// its inner type.
///
/// # Example usage
///
/// ```no_run
/// # use godot::register::{IndirectSignalReceiver, SignalReceiver};
/// # let mut function = || {};
/// # let args = ();
/// IndirectSignalReceiver::from(&mut function)
///     .function()
///     .call((), args);
///```
///
/// # Further reading
/// - [rustc issue #63702](https://github.com/rust-lang/rust/issues/63702)
/// - [identity function trick](https://users.rust-lang.org/t/type-inference-in-closures/78399/3)
/// - [rustc comments around closure type-check](https://github.com/rust-lang/rust/blob/5ad7454f7503b6af2800bf4a7c875962cb03f913/compiler/rustc_hir_typeck/src/fn_ctxt/checks.rs#L306-L317)
pub struct IndirectSignalReceiver<'view, I, Ps, F>
where
    Ps: InParamTuple,
    F: SignalReceiver<I, Ps> + 'static,
{
    inner: &'view mut F,
    _phantoms: PhantomData<(I, Ps)>,
}

impl<'view, I, Ps, F> IndirectSignalReceiver<'view, I, Ps, F>
where
    Ps: InParamTuple,
    F: SignalReceiver<I, Ps> + 'static,
{
    /// Retrieves inner `&mut F` function ready to be used as [`SignalReceiver`].
    pub fn function(&'view mut self) -> &'view mut F {
        self.inner
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Generated impls

macro_rules! impl_signal_recipient {
    ($( $args:ident : $Ps:ident ),*) => {
        // --------------------------------------------------------------------------------------------------------------------------------------
        // SignalReceiver

        // Global and associated functions.
        impl<F, R, $($Ps: std::fmt::Debug + FromGodot + 'static),*> SignalReceiver<(), ( $($Ps,)* )> for F
            where F: FnMut( $($Ps,)* ) -> R + 'static
        {
            fn call(&mut self, _no_instance: (), ($($args,)*): ( $($Ps,)* )) {
                self($($args,)*);
            }
        }

        // Methods with mutable receiver - &mut self.
        impl<F, R, C, $($Ps: std::fmt::Debug + FromGodot + 'static),*> SignalReceiver<&mut C, ( $($Ps,)* )> for F
            where F: FnMut( &mut C, $($Ps,)* ) -> R + 'static
        {
            fn call(&mut self, instance: &mut C, ($($args,)*): ( $($Ps,)* )) {
                self(instance, $($args,)*);
            }
        }

        // Methods with immutable receiver - &self.
        impl<F, R, C, $($Ps: std::fmt::Debug + FromGodot + 'static),*> SignalReceiver<&C, ( $($Ps,)* )> for F
            where F: FnMut( &C, $($Ps,)* ) -> R + 'static
        {
            fn call(&mut self, instance: &C, ($($args,)*): ( $($Ps,)* )) {
                self(instance, $($args,)*);
            }
        }

        // Methods with gd receiver - Gd<Self>.
        impl<F, R, C, $($Ps: std::fmt::Debug + FromGodot + 'static),*> SignalReceiver<Gd<C>, ( $($Ps,)* )> for F
            where F: FnMut( Gd<C>, $($Ps,)* ) -> R + 'static, C: GodotClass
        {
            fn call(&mut self, instance: Gd<C>, ($($args,)*): ( $($Ps,)* )) {
                self(instance, $($args,)*);
            }
        }

                // --------------------------------------------------------------------------------------------------------------------------------------
        // FnMut -> IndirectSignalReceiver

        impl<'c_view, F, R, $($Ps: std::fmt::Debug + FromGodot + 'static),*> From<&'c_view mut F> for IndirectSignalReceiver<'c_view, (), ($($Ps,)*), F>
            where F: FnMut( $($Ps,)* ) -> R + 'static
        {
            fn from(value: &'c_view mut F) -> Self {
                IndirectSignalReceiver {
                    inner: value,
                    _phantoms: PhantomData,
                }
            }
        }

        impl<'c_view, F, R, C, $($Ps: std::fmt::Debug + FromGodot + 'static),*> From<&'c_view mut F> for IndirectSignalReceiver<'c_view, &mut C, ($($Ps,)*), F>
            where F: FnMut( &mut C, $($Ps,)* ) -> R + 'static
        {
            fn from(value: &'c_view mut F) -> Self {
                IndirectSignalReceiver {
                    inner: value,
                    _phantoms: PhantomData,
                }
            }
        }

        impl<'c_view, F, R, C, $($Ps: std::fmt::Debug + FromGodot + 'static),*> From<&'c_view mut F> for IndirectSignalReceiver<'c_view, &C, ($($Ps,)*), F>
            where F: FnMut( &C, $($Ps,)* ) -> R + 'static
        {
            fn from(value: &'c_view mut F) -> Self {
                IndirectSignalReceiver {
                    inner: value,
                    _phantoms: PhantomData,
                }
            }
        }

        impl<'c_view, F, R, C, $($Ps: std::fmt::Debug + FromGodot + 'static),*> From<&'c_view mut F> for IndirectSignalReceiver<'c_view, Gd<C>, ($($Ps,)*), F>
            where F: FnMut( Gd<C>, $($Ps,)* ) -> R + 'static, C: GodotClass
        {
            fn from(value: &'c_view mut F) -> Self {
                IndirectSignalReceiver {
                    inner: value,
                    _phantoms: PhantomData,
                }
            }
        }
    };
}

impl_signal_recipient!();
impl_signal_recipient!(arg0: P0);
impl_signal_recipient!(arg0: P0, arg1: P1);
impl_signal_recipient!(arg0: P0, arg1: P1, arg2: P2);
impl_signal_recipient!(arg0: P0, arg1: P1, arg2: P2, arg3: P3);
impl_signal_recipient!(arg0: P0, arg1: P1, arg2: P2, arg3: P3, arg4: P4);
impl_signal_recipient!(arg0: P0, arg1: P1, arg2: P2, arg3: P3, arg4: P4, arg5: P5);
impl_signal_recipient!(arg0: P0, arg1: P1, arg2: P2, arg3: P3, arg4: P4, arg5: P5, arg6: P6);
impl_signal_recipient!(arg0: P0, arg1: P1, arg2: P2, arg3: P3, arg4: P4, arg5: P5, arg6: P6, arg7: P7);
impl_signal_recipient!(arg0: P0, arg1: P1, arg2: P2, arg3: P3, arg4: P4, arg5: P5, arg6: P6, arg7: P7, arg8: P8);
impl_signal_recipient!(arg0: P0, arg1: P1, arg2: P2, arg3: P3, arg4: P4, arg5: P5, arg6: P6, arg7: P7, arg8: P8, arg9: P9);
