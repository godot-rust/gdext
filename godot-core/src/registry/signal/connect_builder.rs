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
use crate::obj::{bounds, Bounds, Gd, GodotClass, WithSignals};
use crate::registry::signal::{ToSignalObj, TypedSignal};
use std::fmt::Debug;

/// Builder for customizing signal connections.
///
/// Allows a high degree of customization for connecting [`TypedSignal`], while maintaining complete type safety.
///
/// See the [Signals](https://godot-rust.github.io/book/register/signals.html) chapter in the book for a general introduction and examples.
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
/// After customizing your builder, you can register the connection with various `connect_*` functions.
///
/// To connect to **methods** (member functions) with a signal object, you have the following combinations:
///
/// | Signal object | 1st parameter `&mut C`                         | 1st parameter `&mut Gd<C>`                   |
/// |---------------|------------------------------------------------|----------------------------------------------|
/// | `self`        | [`connect_self_mut`][Self::connect_self_mut]   | [`connect_self_gd`][Self::connect_self_gd]   |
/// | other object  | [`connect_other_mut`][Self::connect_other_mut] | [`connect_other_gd`][Self::connect_other_gd] |
///
/// <br>
///
/// For **global functions, associated functions and closures**, you can use the following APIs:
/// - [`connect()`][Self::connect]: Connect any function running on the same thread as the signal emitter.
/// - [`connect_sync()`](#method.connect_sync): Connect a global/associated function or closure that should be callable across threads.
///   Allows signals to be emitted from other threads.
///   - Requires `Send` + `Sync` bounds on the provided function.
///   - Is only available for the Cargo feature `experimental-threads`.
///
/// # Implementation and documentation notes
/// See [`TypedSignal` docs](struct.TypedSignal.html#implementation-and-documentation-notes) for a background on the `connect_*` API design.
///
/// <div class="warning">
/// <strong>Warning:</strong>
/// Exact type parameters are subject to change and not part of the public API. Since it's a type-state API, new states might require new
/// type parameters. Thus, try not to name the <code>ConnectBuilder</code> type in your code; most connection setup doesn't need it.
/// </div>
// If naming the type becomes a requirement, there may be some options:
// - Use a type alias in the module or TypedSignal, exposing only public parameters. This would work for constructor, but not all transformations.
// - Pack multiple types together into "type lists", i.e. custom structs carrying the type state. For a user, this would appear as one type,
// - which could also be #[doc(hidden)]. However, this may make the trait resolution more complex and worsen error messages, so not done now.
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
    ($( #[$attr:meta] )? $( $args:ident : $Ps:ident ),*) => {
        $( #[$attr] )?
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
            /// # Related APIs
            /// - To connect to a method on the object that owns this signal, use [`connect_self_mut()`][Self::connect_self_mut] or
            ///   [`connect_self_gd()`][Self::connect_self_gd].
            /// - To connect to methods on other objects, use [`connect_other_mut()`][Self::connect_other_mut] or
            ///   [`connect_other_gd()`][Self::connect_other_gd].
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

            /// Connect a method with `&mut self` as the first parameter (user classes only).
            ///
            /// # Related APIs
            /// - Use [`connect_self_gd()`][Self::connect_self_gd] to receive `Gd<Self>` instead and avoid implicit `bind_mut()` on emit.  \
            ///   For engine classes, `&mut self` is not supported at all.
            /// - To connect to methods on other objects, use [`connect_other_mut()`][Self::connect_other_mut].
            /// - If you need [connect flags](ConnectFlags), call [`flags()`](Self::flags) before this.
            /// - If you need cross-thread signals, use [`connect_sync()`](#method.connect_sync) instead (requires feature `experimental-threads`).
            pub fn connect_self_mut<F, R>(self, mut function: F)
            where
                C: Bounds<Declarer = bounds::DeclUser>,
                F: FnMut(&mut C, $($Ps),*) -> R + 'static,
            {
                let mut gd = self.parent_sig.receiver_object();
                let godot_fn = make_godot_fn(move |($($args,)*): ($($Ps,)*)| {
                    let mut guard = Gd::bind_mut(&mut gd);
                    function(&mut *guard, $($args),*);
                });

                self.inner_connect_godot_fn::<F>(godot_fn);
            }

            /// Connect a method with `&mut Gd<Self>` as the first parameter (user + engine classes).
            ///
            /// # Related APIs
            /// - If your class `C` is user-defined and you'd like to have an automatic `bind_mut()` and receive `&mut self`, then
            ///   use [`connect_self_mut()`][Self::connect_self_mut] instead.
            /// - To connect to methods on other objects, use [`connect_other_gd()`][Self::connect_other_gd].
            /// - If you need [connect flags](ConnectFlags), call [`flags()`](Self::flags) before this.
            /// - If you need cross-thread signals, use [`connect_sync()`](#method.connect_sync) instead (requires feature `experimental-threads`).
            pub fn connect_self_gd<F, R>(self, mut function: F)
            where
                F: FnMut(&mut Gd<C>, $($Ps),*) -> R + 'static,
            {
                let mut gd = self.parent_sig.receiver_object();
                let godot_fn = make_godot_fn(move |($($args,)*): ($($Ps,)*)| {
                    function(&mut gd, $($args),*);
                });

                self.inner_connect_godot_fn::<F>(godot_fn);
            }

            /// Connect a method with any `&mut OtherC` as the first parameter (user classes only).
            ///
            /// The parameter `object` can be of 2 different "categories":
            /// - Any `&Gd<OtherC>` (e.g.: `&Gd<Node>`, `&Gd<MyClass>`).
            /// - `&OtherC`, as long as `OtherC` is a user class that contains a `Base<T>` field (it implements the
            ///   [`WithBaseField`](crate::obj::WithBaseField) trait).
            ///
            /// # Related APIs
            /// - Use [`connect_other_gd()`][Self::connect_other_gd] to receive `Gd<Self>` instead and avoid implicit `bind_mut()` on emit.  \
            ///   For engine classes, `&mut self` is not supported at all.
            /// - To connect to methods on the object that owns this signal, use [`connect_self_mut()`][Self::connect_self_mut].
            /// - If you need [connect flags](ConnectFlags), call [`flags()`](Self::flags) before this.
            /// - If you need cross-thread signals, use [`connect_sync()`](#method.connect_sync) instead (requires feature "experimental-threads").
            pub fn connect_other_mut<F, R, OtherC>(self, object: &impl ToSignalObj<OtherC>, mut method: F)
            where
                OtherC: GodotClass + Bounds<Declarer = bounds::DeclUser>,
                F: FnMut(&mut OtherC, $($Ps),*) -> R + 'static,
            {
                let mut gd = object.to_signal_obj();

                let godot_fn = make_godot_fn(move |($($args,)*): ($($Ps,)*)| {
                    let mut guard = Gd::bind_mut(&mut gd);
                    method(&mut *guard, $($args),*);
                });

                self.inner_connect_godot_fn::<F>(godot_fn);
            }

            /// Connect a method with any `&mut Gd<OtherC>` as the first parameter (user + engine classes).
            ///
            /// The parameter `object` can be of 2 different "categories":
            /// - Any `&Gd<OtherC>` (e.g.: `&Gd<Node>`, `&Gd<MyClass>`).
            /// - `&OtherC`, as long as `OtherC` is a user class that contains a `Base<T>` field (it implements the
            ///   [`WithBaseField`](crate::obj::WithBaseField) trait).
            ///
            /// # Related APIs
            /// - To connect to methods on the object that owns this signal, use [`connect_self_gd()`][Self::connect_self_gd].
            /// - If you need [connect flags](ConnectFlags), call [`flags()`](Self::flags) before this.
            /// - If you need cross-thread signals, use [`connect_sync()`](#method.connect_sync) instead (requires feature "experimental-threads").
            pub fn connect_other_gd<F, R, OtherC>(self, object: &impl ToSignalObj<OtherC>, mut method: F)
            where
                OtherC: GodotClass,
                F: FnMut(&mut Gd<OtherC>, $($Ps),*) -> R + 'static,
            {
                let mut gd = object.to_signal_obj();

                let godot_fn = make_godot_fn(move |($($args,)*): ($($Ps,)*)| {
                    method(&mut gd, $($args),*);
                });

                self.inner_connect_godot_fn::<F>(godot_fn);
            }

            /// Connect to this signal using a thread-safe function, allows the signal to be called across threads.
            ///
            /// Requires `Send` + `Sync` bounds on the provided function `F`, and is only available for the `experimental-threads`
            /// Cargo feature.
            ///
            /// If you need [connect flags](ConnectFlags), call [`flags()`](Self::flags) before this.
            #[cfg(feature = "experimental-threads")]
            pub fn connect_sync<F, R>(self, mut function: F)
            where
                // Why both Send+Sync: closure can not only impact another thread (Sync), but it's also possible to share such Callables across threads
                // (Send) or even call them from multiple threads (Sync). We don't differentiate the fine-grained needs, it's either thread-safe or not.
                F: FnMut($($Ps),*) -> R + Send + Sync + 'static,
            {
                let godot_fn = make_godot_fn(move |($($args,)*): ($($Ps,)*)| {
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

impl_builder_connect!(#[doc(hidden)] );
impl_builder_connect!(#[doc(hidden)] arg0: P0);
impl_builder_connect!(#[doc(hidden)] arg0: P0, arg1: P1);
impl_builder_connect!(               arg0: P0, arg1: P1, arg2: P2);
impl_builder_connect!(#[doc(hidden)] arg0: P0, arg1: P1, arg2: P2, arg3: P3);
impl_builder_connect!(#[doc(hidden)] arg0: P0, arg1: P1, arg2: P2, arg3: P3, arg4: P4);
impl_builder_connect!(#[doc(hidden)] arg0: P0, arg1: P1, arg2: P2, arg3: P3, arg4: P4, arg5: P5);
impl_builder_connect!(#[doc(hidden)] arg0: P0, arg1: P1, arg2: P2, arg3: P3, arg4: P4, arg5: P5, arg6: P6);
impl_builder_connect!(#[doc(hidden)] arg0: P0, arg1: P1, arg2: P2, arg3: P3, arg4: P4, arg5: P5, arg6: P6, arg7: P7);
impl_builder_connect!(#[doc(hidden)] arg0: P0, arg1: P1, arg2: P2, arg3: P3, arg4: P4, arg5: P5, arg6: P6, arg7: P7, arg8: P8);
impl_builder_connect!(#[doc(hidden)] arg0: P0, arg1: P1, arg2: P2, arg3: P3, arg4: P4, arg5: P5, arg6: P6, arg7: P7, arg8: P8, arg9: P9);
