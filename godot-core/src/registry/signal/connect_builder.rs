/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use super::{make_callable_name, make_godot_fn};
use crate::builtin::{Callable, GString, Variant};
use crate::classes::object::ConnectFlags;
use crate::meta;
use crate::meta::FromGodot;
use crate::obj::WithSignals;
use crate::registry::signal::{GodotDeref, ToSignalObj, TypedSignal};
use std::fmt::Debug;
use std::ops::DerefMut;

/// Builder for customizing signal connections.
///
/// Allows a high degree of customization for connecting signals, while maintaining complete type safety.
///
/// <div class="warning">
/// <strong>Warning:</strong>
/// Exact type parameters are subject to change and not part of the public API. We could annotate <code>#[doc(hidden)]</code>, but it would make
/// things harder to understand. Thus, try not to name the <code>ConnectBuilder</code> type in your code; most connection setup doesn't need it.
/// </div>
// If naming the type becomes a requirement, there may be some options:
// - Use a type alias in the module or TypedSignal, exposing only public parameters. This would work for constructor, but not all transformations.
// - Pack multiple types together into "type lists", i.e. custom structs carrying the type state. For a user, this would appear as one type,
// - which could also be #[doc(hidden)]. However, this may make the trait resolution more complex and worsen error messages, so not done now.
///
/// # Customization
/// Customizing your signal connection must be done **before** providing the function being connected
/// (can be done by using of the `connect_**` methods) (see section `Finalizing` bellow).
///
/// All these methods are optional, and they can be combined.
// Use HTML link due to conditional compilation; renders badly if target symbol is unavailable.
/// - [`name()`][Self::name]: Name of the `Callable` (for debug purposes).  \
///   If not specified, the Rust function name is used. This is typically a good default, but not very readable for closures.
/// - [`flags()`][Self::flags]: Provide one or multiple [`ConnectFlags`][crate::classes::object::ConnectFlags], possibly combined with bitwise OR.
///
/// # Finalizing
/// After customizing your builder, you can register the connection by using one of the following methods:
/// - [`connect()`][Self::connect]: Connect a global/associated function or a closure.
/// - [`connect_self()`][Self::connect_self]: Connect a method or closure that runs on the signal emitter.
/// - [`connect_other()`][Self::connect_other]: Connect a method or closure that runs on a separate object.
/// - [`connect_sync()`](#method.connect_sync): Connect a global/associated function or closure that should be callable across threads. \
///   Allows signal to be emitted from other threads. \
///   Requires `Send` + `Sync` bounds on the provided function/method, and is only available for the `experimental-threads` Cargo feature.
#[must_use]
pub struct ConnectBuilder<'ts, 'c, C: WithSignals, Ps> {
    parent_sig: &'ts TypedSignal<'c, C, Ps>,
    data: BuilderData,
}

/// Gathers all the non-typestate data, so that the builder can easily transfer it without manually moving each field.
#[derive(Default)]
struct BuilderData {
    /// User-specified name; if not provided, the Rust RTTI type name of the function is used.
    callable_name: Option<GString>,

    /// Godot connection flags.
    connect_flags: Option<ConnectFlags>,
}

#[allow(clippy::needless_lifetimes)] // 'ts + 'c are used conditionally.
impl<'ts, 'c, C, Ps> ConnectBuilder<'ts, 'c, C, Ps>
where
    C: WithSignals,
    Ps: meta::ParamTuple,
{
    pub(super) fn new(parent_sig: &'ts TypedSignal<'c, C, Ps>) -> Self {
        ConnectBuilder {
            parent_sig,
            data: BuilderData::default(),
        }
    }

    /// Name of the `Callable`, mostly used for debugging.
    ///
    /// If not provided, the Rust type name of the function/method is used.
    pub fn name(mut self, name: impl meta::AsArg<GString>) -> Self {
        assert!(
            self.data.callable_name.is_none(),
            "name() called twice on the same builder."
        );

        meta::arg_into_owned!(name);
        self.data.callable_name = Some(name);
        self
    }

    /// Add one or multiple flags to the connection, possibly combined with `|` operator.
    pub fn flags(mut self, flags: ConnectFlags) -> Self {
        assert!(
            self.data.connect_flags.is_none(),
            "flags() called twice on the same builder."
        );

        self.data.connect_flags = Some(flags);
        self
    }

    /// Directly connect a Rust callable `godot_fn`, with a name based on `F`.
    ///
    /// This exists as a shorthand for the connect methods and avoids the generic instantiation of the full-blown
    /// type state builder for simple + common connections, thus hopefully being a tiny bit lighter on compile times.
    fn inner_connect_godot_fn<F>(
        self,
        godot_fn: impl FnMut(&[&Variant]) -> Result<Variant, ()> + 'static,
    ) {
        let callable_name = match &self.data.callable_name {
            Some(user_provided_name) => user_provided_name,
            None => &make_callable_name::<F>(),
        };

        let callable = Callable::from_local_fn(callable_name, godot_fn);
        self.parent_sig
            .inner_connect_untyped(&callable, self.data.connect_flags);
    }
}

macro_rules! impl_builder_connect {
    ($( $args:ident : $Ps:ident ),*) => {
        // --------------------------------------------------------------------------------------------------------------------------------------
        // SignalReceiver

        impl<C: WithSignals, $($Ps: Debug + FromGodot + 'static),*>
            ConnectBuilder<'_, '_, C, ($($Ps,)*)> {
            /// Connect a non-member function (global function, associated function or closure).
            ///
            /// Example usages:
            /// ```ignore
            /// sig.connect_builder().connect(Self::static_func);
            /// sig.connect_builder().flags(ConnectFlags::DEFERRED).connect(global_func);
            /// sig.connect(|arg| { /* closure */ });
            /// ```
            ///
            /// - To connect to a method on the object that owns this signal, use [`connect_self()`][Self::connect_self].
            /// - If you need [`connect flags`](ConnectFlags), call [`flags()`](Self::flags) before this.
            /// - If you need cross-thread signals, use [`connect_sync()`](#method.connect_sync) instead (requires feature "experimental-threads").
            pub fn connect<F, R>(self, mut function: F)
            where
                F: FnMut($($Ps),*) -> R + 'static,
            {
                let godot_fn = make_godot_fn(move |($($args,)*):($($Ps,)*)| {
                    function($($args),*);
                });

                self.inner_connect_godot_fn::<F>(godot_fn);
            }

            /// Connect a method (member function) with `&mut self` as the first parameter.
            ///
            /// - To connect to methods on other objects, use [`connect_other()`][Self::connect_other].
            /// - If you need [`connect flags`](ConnectFlags), call [`flags()`](Self::flags) before this.
            /// - If you need cross-thread signals, use [`connect_sync()`](#method.connect_sync) instead (requires feature "experimental-threads").
            pub fn connect_self<F, R, Decl>(self, mut function: F)
            where
                F: FnMut(&mut C, $($Ps),*) -> R + 'static,
                C: GodotDeref<Decl>,
            {
                let mut gd = self.parent_sig.receiver_object();
                let godot_fn = make_godot_fn(move |($($args,)*):($($Ps,)*)| {
                    let mut target = C::get_mut(&mut gd);
                    let target_mut = target.deref_mut();
                    function(target_mut, $($args),*);
                });

                self.inner_connect_godot_fn::<F>(godot_fn);
            }

            /// Connect a method (member function) with any `&mut OtherC` as the first parameter, where
            /// `OtherC`: [`GodotClass`](crate::obj::GodotClass) (both user and engine classes are accepted).
            ///
            /// The parameter `object` can be of 2 different "categories":
            /// - Any `&Gd<OtherC>` (e.g.: `&Gd<Node>`, `&Gd<CustomUserClass>`).
            /// - `&OtherC`, as long as `OtherC` is a user class that contains a `base` field (it implements the
            ///   [`WithBaseField`](crate::obj::WithBaseField) trait).
            ///
            /// ---
            ///
            /// - To connect to methods on the object that owns this signal, use [`connect_self()`][Self::connect_self].
            /// - If you need [`connect flags`](ConnectFlags), call [`flags()`](Self::flags) before this.
            /// - If you need cross-thread signals, use [`connect_sync()`](#method.connect_sync) instead (requires feature "experimental-threads").
            pub fn connect_other<F, R, OtherC, Decl>(self, object: &impl ToSignalObj<OtherC>, mut method: F)
            where
                F: FnMut(&mut OtherC, $($Ps),*) -> R + 'static,
                OtherC: GodotDeref<Decl>,
            {
                let mut gd = object.to_signal_obj();

                let godot_fn = make_godot_fn(move |($($args,)*):($($Ps,)*)| {
                    let mut target = OtherC::get_mut(&mut gd);
                    let target_mut = target.deref_mut();
                    method(target_mut, $($args),*);
                });

                self.inner_connect_godot_fn::<F>(godot_fn);
            }

            /// Connect to this signal using a thread-safe function, allows the signal to be called across threads.
            ///
            /// Requires `Send` + `Sync` bounds on the provided function `F`, and is only available for the `experimental-threads`
            /// Cargo feature.
            ///
            /// If you need [`connect flags`](ConnectFlags), call [`flags()`](Self::flags) before this.
            #[cfg(feature = "experimental-threads")]
            pub fn connect_sync<F, R>(self, mut function: F)
            where
                // Why both Send+Sync: closure can not only impact another thread (Sync), but it's also possible to share such Callables across threads
                // (Send) or even call them from multiple threads (Sync). We don't differentiate the fine-grained needs, it's either thread-safe or not.
                F: FnMut($($Ps),*) -> R + Send + Sync + 'static,
            {
                let godot_fn = make_godot_fn(move |($($args,)*):($($Ps,)*)| {
                    function($($args),*);
                });

                let callable_name = match &self.data.callable_name {
                    Some(user_provided_name) => user_provided_name,
                    None => &make_callable_name::<F>(),
                };

                let callable = Callable::from_sync_fn(callable_name, godot_fn);
                self.parent_sig.inner_connect_untyped(&callable, self.data.connect_flags);
            }
        }
    };
}

impl_builder_connect!();
impl_builder_connect!(arg0: P0);
impl_builder_connect!(arg0: P0, arg1: P1);
impl_builder_connect!(arg0: P0, arg1: P1, arg2: P2);
impl_builder_connect!(arg0: P0, arg1: P1, arg2: P2, arg3: P3);
impl_builder_connect!(arg0: P0, arg1: P1, arg2: P2, arg3: P3, arg4: P4);
impl_builder_connect!(arg0: P0, arg1: P1, arg2: P2, arg3: P3, arg4: P4, arg5: P5);
impl_builder_connect!(arg0: P0, arg1: P1, arg2: P2, arg3: P3, arg4: P4, arg5: P5, arg6: P6);
impl_builder_connect!(arg0: P0, arg1: P1, arg2: P2, arg3: P3, arg4: P4, arg5: P5, arg6: P6, arg7: P7);
impl_builder_connect!(arg0: P0, arg1: P1, arg2: P2, arg3: P3, arg4: P4, arg5: P5, arg6: P6, arg7: P7, arg8: P8);
impl_builder_connect!(arg0: P0, arg1: P1, arg2: P2, arg3: P3, arg4: P4, arg5: P5, arg6: P6, arg7: P7, arg8: P8, arg9: P9);
