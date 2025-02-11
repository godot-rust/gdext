/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::fmt;
use std::ptr;

use godot_ffi as sys;

use crate::builtin::{inner, Array, Callable, Dictionary, StringName, Variant};
use crate::classes::Object;
use crate::global::Error;
use crate::meta;
use crate::meta::{FromGodot, GodotType, ToGodot};
use crate::obj::bounds::DynMemory;
use crate::obj::{Bounds, Gd, GodotClass, InstanceId};
use sys::{ffi_methods, GodotFfi};

#[cfg(since_api = "4.2")]
pub use futures::*;

/// A `Signal` represents a signal of an Object instance in Godot.
///
/// Signals are composed of a reference to an `Object` and the name of the signal on this object.
///
/// # Godot docs
///
/// [`Signal` (stable)](https://docs.godotengine.org/en/stable/classes/class_signal.html)
pub struct Signal {
    opaque: sys::types::OpaqueSignal,
}

impl Signal {
    fn from_opaque(opaque: sys::types::OpaqueSignal) -> Self {
        Self { opaque }
    }

    /// Create a signal for the signal `object::signal_name`.
    ///
    /// _Godot equivalent: `Signal(Object object, StringName signal)`_
    pub fn from_object_signal<T, S>(object: &Gd<T>, signal_name: S) -> Self
    where
        T: GodotClass,
        S: meta::AsArg<StringName>,
    {
        meta::arg_into_ref!(signal_name);

        unsafe {
            Self::new_with_uninit(|self_ptr| {
                let ctor = sys::builtin_fn!(signal_from_object_signal);
                let raw = object.to_ffi();
                let args = [raw.as_arg_ptr(), signal_name.sys()];
                ctor(self_ptr, args.as_ptr());
            })
        }
    }

    /// Creates an invalid/empty signal that cannot be called.
    ///
    /// _Godot equivalent: `Signal()`_
    pub fn invalid() -> Self {
        unsafe {
            Self::new_with_uninit(|self_ptr| {
                let ctor = sys::builtin_fn!(signal_construct_default);
                ctor(self_ptr, ptr::null_mut())
            })
        }
    }

    /// Connects this signal to the specified callable.
    ///
    /// Optional flags can be also added to configure the connection's behavior (see [`ConnectFlags`](crate::classes::object::ConnectFlags) constants).
    /// You can provide additional arguments to the connected callable by using `Callable::bind`.
    ///
    /// A signal can only be connected once to the same [`Callable`]. If the signal is already connected,
    /// returns [`Error::ERR_INVALID_PARAMETER`] and
    /// pushes an error message, unless the signal is connected with [`ConnectFlags::REFERENCE_COUNTED`](crate::classes::object::ConnectFlags::REFERENCE_COUNTED).
    /// To prevent this, use [`Self::is_connected`] first to check for existing connections.
    pub fn connect(&self, callable: &Callable, flags: i64) -> Error {
        let error = self.as_inner().connect(callable, flags);

        Error::from_godot(error as i32)
    }

    /// Disconnects this signal from the specified [`Callable`].
    ///
    /// If the connection does not exist, generates an error. Use [`Self::is_connected`] to make sure that the connection exists.
    pub fn disconnect(&self, callable: &Callable) {
        self.as_inner().disconnect(callable);
    }

    /// Emits this signal.
    ///
    /// All Callables connected to this signal will be triggered.
    pub fn emit(&self, varargs: &[Variant]) {
        let Some(mut object) = self.object() else {
            return;
        };

        object.emit_signal(&self.name(), varargs);
    }

    /// Returns an [`Array`] of connections for this signal.
    ///
    /// Each connection is represented as a Dictionary that contains three entries:
    ///  - `signal` is a reference to this [`Signal`];
    ///  - `callable` is a reference to the connected [`Callable`];
    ///  - `flags` is a combination of [`ConnectFlags`](crate::classes::object::ConnectFlags).
    ///
    /// _Godot equivalent: `get_connections`_
    pub fn connections(&self) -> Array<Dictionary> {
        self.as_inner()
            .get_connections()
            .iter_shared()
            .map(|variant| variant.to())
            .collect()
    }

    /// Returns the name of the signal.
    pub fn name(&self) -> StringName {
        self.as_inner().get_name()
    }

    /// Returns the object to which this signal belongs.
    ///
    /// Returns [`None`] when this signal doesn't have any object, or the object is dead. You can differentiate these two situations using
    /// [`object_id()`][Self::object_id].
    ///
    /// _Godot equivalent: `get_object`_
    pub fn object(&self) -> Option<Gd<Object>> {
        self.as_inner().get_object().map(|mut object| {
            <Object as Bounds>::DynMemory::maybe_inc_ref(&mut object.raw);
            object
        })
    }

    /// Returns the ID of this signal's object, see also [`Gd::instance_id`].
    ///
    /// Returns [`None`] when this signal doesn't have any object.
    ///
    /// If the pointed-to object is dead, the ID will still be returned. Use [`object()`][Self::object] to check for liveness.
    ///
    /// _Godot equivalent: `get_object_id`_
    pub fn object_id(&self) -> Option<InstanceId> {
        let id = self.as_inner().get_object_id();
        InstanceId::try_from_i64(id)
    }

    /// Returns `true` if the specified [`Callable`] is connected to this signal.
    pub fn is_connected(&self, callable: &Callable) -> bool {
        self.as_inner().is_connected(callable)
    }

    /// Returns `true` if the signal's name does not exist in its object, or the object is not valid.
    pub fn is_null(&self) -> bool {
        self.as_inner().is_null()
    }

    #[doc(hidden)]
    pub fn as_inner(&self) -> inner::InnerSignal {
        inner::InnerSignal::from_outer(self)
    }
}

// SAFETY:
// The `opaque` in `Signal` is just a pair of pointers, and requires no special initialization or cleanup
// beyond what is done in `from_opaque` and `drop`. So using `*mut Opaque` is safe.
unsafe impl GodotFfi for Signal {
    fn variant_type() -> sys::VariantType {
        sys::VariantType::SIGNAL
    }

    ffi_methods! { type sys::GDExtensionTypePtr = *mut Opaque;
        fn new_from_sys;
        fn new_with_uninit;
        fn from_arg_ptr;
        fn sys;
        fn sys_mut;
        fn move_return_ptr;
    }

    unsafe fn new_with_init(init_fn: impl FnOnce(sys::GDExtensionTypePtr)) -> Self {
        let mut result = Self::invalid();
        init_fn(result.sys_mut());
        result
    }
}

impl_builtin_traits! {
    for Signal {
        Clone => signal_construct_copy;
        Drop => signal_destroy;
        PartialEq => signal_operator_equal;
    }
}

crate::meta::impl_godot_as_self!(Signal);

impl fmt::Debug for Signal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = self.name();
        let object = self.object();

        f.debug_struct("signal")
            .field("name", &name)
            .field("object", &object)
            .finish()
    }
}

impl fmt::Display for Signal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_variant())
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------
// Implementation of a rust future for Godot Signals
#[cfg(since_api = "4.2")]
mod futures {
    use std::fmt::Display;
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::{Arc, Mutex};
    use std::task::{Context, Poll, Waker};

    use crate::builtin::{Callable, RustCallable, Variant};
    use crate::classes::object::ConnectFlags;
    use crate::meta::FromGodot;
    use crate::obj::EngineEnum;

    use super::Signal;

    pub struct SignalFuture<R: FromSignalArgs> {
        state: Arc<Mutex<(Option<R>, Option<Waker>)>>,
        callable: Callable,
        signal: Signal,
    }

    impl<R: FromSignalArgs> SignalFuture<R> {
        fn new(signal: Signal) -> Self {
            let state = Arc::new(Mutex::new((None, Option::<Waker>::None)));
            let callback_state = state.clone();

            // the callable currently requires that the return value is Sync + Send
            let callable = Callable::from_local_fn("async_task", move |args: &[&Variant]| {
                let mut lock = callback_state.lock().unwrap();
                let waker = lock.1.take();

                lock.0.replace(R::from_args(args));
                drop(lock);

                if let Some(waker) = waker {
                    waker.wake();
                }

                Ok(Variant::nil())
            });

            signal.connect(&callable, ConnectFlags::ONE_SHOT.ord() as i64);

            Self {
                state,
                callable,
                signal,
            }
        }
    }

    impl<R: FromSignalArgs> Future for SignalFuture<R> {
        type Output = R;

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            let mut lock = self.state.lock().unwrap();

            if let Some(result) = lock.0.take() {
                return Poll::Ready(result);
            }

            lock.1.replace(cx.waker().clone());

            Poll::Pending
        }
    }

    impl<R: FromSignalArgs> Drop for SignalFuture<R> {
        fn drop(&mut self) {
            if !self.callable.is_valid() {
                return;
            }

            if self.signal.object().is_none() {
                return;
            }

            if self.signal.is_connected(&self.callable) {
                self.signal.disconnect(&self.callable);
            }
        }
    }

    struct GuaranteedSignalFutureResolver<R> {
        state: Arc<Mutex<(GuaranteedSignalFutureState<R>, Option<Waker>)>>,
    }

    impl<R> Clone for GuaranteedSignalFutureResolver<R> {
        fn clone(&self) -> Self {
            Self {
                state: self.state.clone(),
            }
        }
    }

    impl<R> GuaranteedSignalFutureResolver<R> {
        fn new(state: Arc<Mutex<(GuaranteedSignalFutureState<R>, Option<Waker>)>>) -> Self {
            Self { state }
        }
    }

    impl<R> std::hash::Hash for GuaranteedSignalFutureResolver<R> {
        fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
            state.write_usize(Arc::as_ptr(&self.state) as usize);
        }
    }

    impl<R> PartialEq for GuaranteedSignalFutureResolver<R> {
        fn eq(&self, other: &Self) -> bool {
            Arc::ptr_eq(&self.state, &other.state)
        }
    }

    impl<R: FromSignalArgs> RustCallable for GuaranteedSignalFutureResolver<R> {
        fn invoke(&mut self, args: &[&Variant]) -> Result<Variant, ()> {
            let mut lock = self.state.lock().unwrap();
            let waker = lock.1.take();

            lock.0 = GuaranteedSignalFutureState::Ready(R::from_args(args));
            drop(lock);

            if let Some(waker) = waker {
                waker.wake();
            }

            Ok(Variant::nil())
        }
    }

    impl<R> Display for GuaranteedSignalFutureResolver<R> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "GuaranteedSignalFutureResolver::<{}>",
                std::any::type_name::<R>()
            )
        }
    }

    // this resolver will resolve the future when it's being dropped (i.e. the engine removes all connected signal callables). This is very unusual.
    impl<R> Drop for GuaranteedSignalFutureResolver<R> {
        fn drop(&mut self) {
            let mut lock = self.state.lock().unwrap();

            if !matches!(lock.0, GuaranteedSignalFutureState::Pending) {
                return;
            }

            lock.0 = GuaranteedSignalFutureState::Dead;

            if let Some(ref waker) = lock.1 {
                waker.wake_by_ref();
            }
        }
    }

    #[derive(Default)]
    enum GuaranteedSignalFutureState<T> {
        #[default]
        Pending,
        Ready(T),
        Dead,
        Dropped,
    }

    impl<T> GuaranteedSignalFutureState<T> {
        fn take(&mut self) -> Self {
            let new_value = match self {
                Self::Pending => Self::Pending,
                Self::Ready(_) | Self::Dead => Self::Dead,
                Self::Dropped => Self::Dropped,
            };

            std::mem::replace(self, new_value)
        }
    }

    /// The guaranteed signal future will always resolve, but might resolve to `None` if the owning object is freed
    /// before the signal is emitted.
    ///
    /// This is inconsistent with how awaiting signals in Godot work and how async works in rust. The behavior was requested as part of some
    /// user feedback for the initial POC.
    pub struct GuaranteedSignalFuture<R: FromSignalArgs> {
        state: Arc<Mutex<(GuaranteedSignalFutureState<R>, Option<Waker>)>>,
        callable: GuaranteedSignalFutureResolver<R>,
        signal: Signal,
    }

    impl<R: FromSignalArgs> GuaranteedSignalFuture<R> {
        fn new(signal: Signal) -> Self {
            let state = Arc::new(Mutex::new((
                GuaranteedSignalFutureState::Pending,
                Option::<Waker>::None,
            )));

            // the callable currently requires that the return value is Sync + Send
            let callable = GuaranteedSignalFutureResolver::new(state.clone());

            signal.connect(
                &Callable::from_custom(callable.clone()),
                ConnectFlags::ONE_SHOT.ord() as i64,
            );

            Self {
                state,
                callable,
                signal,
            }
        }
    }

    impl<R: FromSignalArgs> Future for GuaranteedSignalFuture<R> {
        type Output = Option<R>;

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            let mut lock = self.state.lock().unwrap();

            lock.1.replace(cx.waker().clone());

            let value = lock.0.take();

            match value {
                GuaranteedSignalFutureState::Pending => Poll::Pending,
                GuaranteedSignalFutureState::Dropped => unreachable!(),
                GuaranteedSignalFutureState::Dead => Poll::Ready(None),
                GuaranteedSignalFutureState::Ready(value) => Poll::Ready(Some(value)),
            }
        }
    }

    impl<R: FromSignalArgs> Drop for GuaranteedSignalFuture<R> {
        fn drop(&mut self) {
            if self.signal.object().is_none() {
                return;
            }

            self.state.lock().unwrap().0 = GuaranteedSignalFutureState::Dropped;

            let gd_callable = Callable::from_custom(self.callable.clone());

            if self.signal.is_connected(&gd_callable) {
                self.signal.disconnect(&gd_callable);
            }
        }
    }

    pub trait FromSignalArgs: Sync + Send + 'static {
        fn from_args(args: &[&Variant]) -> Self;
    }

    impl<R: FromGodot + Sync + Send + 'static> FromSignalArgs for R {
        fn from_args(args: &[&Variant]) -> Self {
            args.first()
                .map(|arg| (*arg).to_owned())
                .unwrap_or_default()
                .to()
        }
    }

    // more of these should be generated via macro to support more than two signal arguments
    impl<R1: FromGodot + Sync + Send + 'static, R2: FromGodot + Sync + Send + 'static>
        FromSignalArgs for (R1, R2)
    {
        fn from_args(args: &[&Variant]) -> Self {
            (args[0].to(), args[0].to())
        }
    }

    impl Signal {
        pub fn to_guaranteed_future<R: FromSignalArgs>(&self) -> GuaranteedSignalFuture<R> {
            GuaranteedSignalFuture::new(self.clone())
        }

        pub fn to_future<R: FromSignalArgs>(&self) -> SignalFuture<R> {
            SignalFuture::new(self.clone())
        }
    }

    #[cfg(test)]
    mod tests {
        use std::{
            hash::{DefaultHasher, Hash, Hasher},
            sync::Arc,
        };

        use super::GuaranteedSignalFutureResolver;

        #[test]
        fn guaranteed_future_waker_cloned_hash() {
            let waker_a = GuaranteedSignalFutureResolver::<u8>::new(Arc::default());
            let waker_b = waker_a.clone();

            let mut hasher = DefaultHasher::new();
            waker_a.hash(&mut hasher);
            let hash_a = hasher.finish();

            let mut hasher = DefaultHasher::new();
            waker_b.hash(&mut hasher);
            let hash_b = hasher.finish();

            assert_eq!(hash_a, hash_b);
        }
    }
}
