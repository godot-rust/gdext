/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use core::panic;
use std::fmt::Display;
use std::future::{Future, IntoFuture};
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};
use std::thread::ThreadId;

use crate::builtin::{Callable, RustCallable, Signal, Variant};
use crate::classes::object::ConnectFlags;
use crate::global::godot_error;
use crate::meta::sealed::Sealed;
use crate::meta::InParamTuple;
use crate::obj::{Gd, GodotClass, WithSignals};
use crate::registry::signal::TypedSignal;
use crate::sys;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Internal re-exports
#[rustfmt::skip] // Do not reorder.
pub(crate) use crate::impl_dynamic_send;

/// The panicking counter part to the [`FallibleSignalFuture`].
///
/// This future works in the same way as `FallibleSignalFuture`, but panics when the signal object is freed, instead of resolving to a
/// [`Result::Err`].
///
/// # Panics
/// - If the signal object is freed before the signal has been emitted.
/// - If one of the signal arguments is `!Send`, but the signal was emitted on a different thread.
/// - The future's `Drop` implementation can cause a non-unwinding panic in rare cases, should the signal object be freed at the same time
///   as the future is dropped. Make sure to keep signal objects alive until there are no pending futures anymore.
pub struct SignalFuture<R: InParamTuple + IntoDynamicSend>(FallibleSignalFuture<R>);

impl<R: InParamTuple + IntoDynamicSend> SignalFuture<R> {
    fn new(signal: Signal) -> Self {
        Self(FallibleSignalFuture::new(signal))
    }
}

impl<R: InParamTuple + IntoDynamicSend> Future for SignalFuture<R> {
    type Output = R;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let poll_result = self.get_mut().0.poll(cx);

        match poll_result {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Ok(value)) => Poll::Ready(value),
            Poll::Ready(Err(FallibleSignalFutureError)) => panic!(
                "the signal object of a SignalFuture was freed, while the future was still waiting for the signal to be emitted"
            ),
        }
    }
}

// Not derived, otherwise an extra bound `Output: Default` is required.
struct SignalFutureData<T> {
    state: SignalFutureState<T>,
    waker: Option<Waker>,
}

impl<T> Default for SignalFutureData<T> {
    fn default() -> Self {
        Self {
            state: Default::default(),
            waker: None,
        }
    }
}

// Only public for itest.
pub struct SignalFutureResolver<R: IntoDynamicSend> {
    data: Arc<Mutex<SignalFutureData<R::Target>>>,
}

impl<R: IntoDynamicSend> Clone for SignalFutureResolver<R> {
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
        }
    }
}

/// For itest to construct and test a resolver.
#[cfg(feature = "trace")]
pub fn create_test_signal_future_resolver<R: IntoDynamicSend>() -> SignalFutureResolver<R> {
    SignalFutureResolver {
        data: Arc::new(Mutex::new(SignalFutureData::default())),
    }
}

impl<R: IntoDynamicSend> SignalFutureResolver<R> {
    fn new(data: Arc<Mutex<SignalFutureData<R::Target>>>) -> Self {
        Self { data }
    }
}

impl<R: IntoDynamicSend> std::hash::Hash for SignalFutureResolver<R> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_usize(Arc::as_ptr(&self.data) as usize);
    }
}

impl<R: IntoDynamicSend> PartialEq for SignalFutureResolver<R> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.data, &other.data)
    }
}

impl<R: InParamTuple + IntoDynamicSend> RustCallable for SignalFutureResolver<R> {
    fn invoke(&mut self, args: &[&Variant]) -> Variant {
        let waker = {
            let mut data = self.data.lock().unwrap();
            data.state = SignalFutureState::Ready(R::from_variant_array(args).into_dynamic_send());

            // We no longer need the waker after we resolved. If the future is polled again, we'll also get a new waker.
            data.waker.take()
        };

        if let Some(waker) = waker {
            waker.wake();
        }

        Variant::nil()
    }
}

impl<R: IntoDynamicSend> Display for SignalFutureResolver<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SignalFutureResolver::<{}>", std::any::type_name::<R>())
    }
}

// This resolver will change the futures state when it's being dropped (i.e. the engine removes all connected signal callables). By marking
// the future as dead we can resolve it to an error value the next time it gets polled.
impl<R: IntoDynamicSend> Drop for SignalFutureResolver<R> {
    fn drop(&mut self) {
        let mut data = self.data.lock().unwrap();

        if !matches!(data.state, SignalFutureState::Pending) {
            // The future is no longer pending, so no clean up is required.
            return;
        }

        // We mark the future as dead, so the next time it gets polled we can react to it's inability to resolve.
        data.state = SignalFutureState::Dead;

        // If we got a waker we trigger it to get the future polled. If there is no waker, then the future has not been polled yet and we
        // simply wait for the runtime to perform the first poll.
        if let Some(ref waker) = data.waker {
            waker.wake_by_ref();
        }
    }
}

#[derive(Default)]
enum SignalFutureState<T> {
    #[default]
    Pending,
    Ready(T),
    Dead,
    Dropped,
}

impl<T> SignalFutureState<T> {
    fn take(&mut self) -> Self {
        let new_value = match self {
            Self::Pending => Self::Pending,
            Self::Ready(_) | Self::Dead => Self::Dead,
            Self::Dropped => Self::Dropped,
        };

        std::mem::replace(self, new_value)
    }
}

/// A future that tries to resolve as soon as the provided Godot signal was emitted.
///
/// The future might resolve to an error if the signal object is freed before the signal is emitted.
///
/// # Panics
/// - If one of the signal arguments is `!Send`, but the signal was emitted on a different thread.
/// - The future's `Drop` implementation can cause a non-unwinding panic in rare cases, should the signal object be freed at the same time
///   as the future is dropped. Make sure to keep signal objects alive until there are no pending futures anymore.
pub struct FallibleSignalFuture<R: InParamTuple + IntoDynamicSend> {
    data: Arc<Mutex<SignalFutureData<R::Target>>>,
    callable: SignalFutureResolver<R>,
    signal: Signal,
}

impl<R: InParamTuple + IntoDynamicSend> FallibleSignalFuture<R> {
    fn new(signal: Signal) -> Self {
        sys::strict_assert!(
            !signal.is_null(),
            "Failed to create future for invalid signal:\n\
            Either the signal object was already freed, or it\n\
            was not registered in the object before being used.",
        );

        let data = Arc::new(Mutex::new(SignalFutureData::default()));

        // The callable currently requires that the return value is Sync + Send.
        let callable = SignalFutureResolver::new(data.clone());

        signal.connect_flags(
            &Callable::from_custom(callable.clone()),
            ConnectFlags::ONE_SHOT,
        );

        Self {
            data,
            callable,
            signal,
        }
    }

    fn poll(&mut self, cx: &mut Context<'_>) -> Poll<Result<R, FallibleSignalFutureError>> {
        let mut data = self.data.lock().unwrap();

        data.waker.replace(cx.waker().clone());

        let value = data.state.take();

        // Drop the data mutex lock to prevent the mutext from getting poisoned by the potential later panic.
        drop(data);

        match value {
            SignalFutureState::Pending => Poll::Pending,
            SignalFutureState::Dropped => unreachable!(),
            SignalFutureState::Dead => Poll::Ready(Err(FallibleSignalFutureError)),
            SignalFutureState::Ready(value) => {
                let Some(value) = DynamicSend::extract_if_safe(value) else {
                    panic!("the awaited signal was not emitted on the main-thread, but contained a non Send argument");
                };

                Poll::Ready(Ok(value))
            }
        }
    }
}

/// Error that might be returned  by the [`FallibleSignalFuture`].
///
/// This error is being resolved to when the signal object is freed before the awaited singal is emitted.
#[derive(Debug)]
pub struct FallibleSignalFutureError;

impl Display for FallibleSignalFutureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "The signal object was freed before the awaited signal was emitted"
        )
    }
}

impl std::error::Error for FallibleSignalFutureError {}

impl<R: InParamTuple + IntoDynamicSend> Future for FallibleSignalFuture<R> {
    type Output = Result<R, FallibleSignalFutureError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.get_mut().poll(cx)
    }
}

impl<R: InParamTuple + IntoDynamicSend> Drop for FallibleSignalFuture<R> {
    fn drop(&mut self) {
        // The callable might alredy be destroyed, this occurs during engine shutdown.
        if self.signal.is_null() {
            return;
        }

        let mut data_lock = self.data.lock().unwrap();

        data_lock.state = SignalFutureState::Dropped;

        drop(data_lock);

        // We create a new Godot Callable from our RustCallable so we get independent reference counting.
        let gd_callable = Callable::from_custom(self.callable.clone());

        // is_connected will return true if the signal was never emited before the future is dropped.
        //
        // There is a TOCTOU issue here that can occur when the FallibleSignalFuture is dropped at the same time as the signal object is
        // freed on a different thread.
        // We check in the beginning if the signal object is still alive, and we check here again, but the signal object still can be freed
        // between our check and our usage of the object in `is_connected` and `disconnect`. The race condition will manifest in a
        // non-unwinding panic that is hard to track down.
        if !self.signal.is_null() && self.signal.is_connected(&gd_callable) {
            self.signal.disconnect(&gd_callable);
        }
    }
}

impl Signal {
    /// Creates a fallible future for this signal.
    ///
    /// The future will resolve the next time the signal is emitted.
    /// See [`FallibleSignalFuture`] for details.
    ///
    /// Since the `Signal` type does not contain information on the signal argument types, the future output type has to be inferred from
    /// the call to this function.
    pub fn to_fallible_future<R: InParamTuple + IntoDynamicSend>(&self) -> FallibleSignalFuture<R> {
        FallibleSignalFuture::new(self.clone())
    }

    /// Creates a future for this signal.
    ///
    /// The future will resolve the next time the signal is emitted, but might panic if the signal object is freed.
    /// See [`SignalFuture`] for details.
    ///
    /// Since the `Signal` type does not contain information on the signal argument types, the future output type has to be inferred from
    /// the call to this function.
    pub fn to_future<R: InParamTuple + IntoDynamicSend>(&self) -> SignalFuture<R> {
        SignalFuture::new(self.clone())
    }
}

impl<C: WithSignals, R: InParamTuple + IntoDynamicSend> TypedSignal<'_, C, R> {
    /// Creates a fallible future for this signal.
    ///
    /// The future will resolve the next time the signal is emitted.
    /// See [`FallibleSignalFuture`] for details.
    pub fn to_fallible_future(&self) -> FallibleSignalFuture<R> {
        FallibleSignalFuture::new(self.to_untyped())
    }

    /// Creates a future for this signal.
    ///
    /// The future will resolve the next time the signal is emitted, but might panic if the signal object is freed.
    /// See [`SignalFuture`] for details.
    pub fn to_future(&self) -> SignalFuture<R> {
        SignalFuture::new(self.to_untyped())
    }
}

impl<C: WithSignals, R: InParamTuple + IntoDynamicSend> IntoFuture for &TypedSignal<'_, C, R> {
    type Output = R;

    type IntoFuture = SignalFuture<R>;

    fn into_future(self) -> Self::IntoFuture {
        self.to_future()
    }
}

/// Convert a value into a type that is [`Send`] at compile-time while the value might not be.
///
/// This allows to turn any implementor into a type that is `Send`, but requires to also implement [`DynamicSend`] as well.
/// The later trait will verify if a value can actually be sent between threads at runtime.
pub trait IntoDynamicSend: Sealed + 'static {
    type Target: DynamicSend<Inner = Self>;

    fn into_dynamic_send(self) -> Self::Target;
}

/// Runtime-checked `Send` capability.
///
/// Implemented for types that need a static `Send` bound, but where it is determined at runtime whether sending a value was
/// actually safe. Only allows to extract the value if sending across threads is safe, thus fulfilling the `Send` supertrait.
///
/// # Safety
/// The implementor has to guarantee that `extract_if_safe` returns `None`, if the value has been sent between threads while being `!Send`.
///
/// To uphold the `Send` supertrait guarantees, no public API apart from `extract_if_safe` must exist that would give access to the inner value from another thread.
pub unsafe trait DynamicSend: Send + Sealed {
    type Inner;

    fn extract_if_safe(self) -> Option<Self::Inner>;
}

/// Value that can be sent across threads, but only accessed on its original thread.
///
/// When moved to another thread, the inner value can no longer be accessed and will be leaked when the `ThreadConfined` is dropped.
pub struct ThreadConfined<T> {
    value: Option<T>,
    thread_id: ThreadId,
}

// SAFETY: This type can always be sent across threads, but the inner value can only be accessed on its original thread.
unsafe impl<T> Send for ThreadConfined<T> {}

impl<T> ThreadConfined<T> {
    pub(crate) fn new(value: T) -> Self {
        Self {
            value: Some(value),
            thread_id: std::thread::current().id(),
        }
    }

    /// Retrieve the inner value, if the current thread is the one in which the `ThreadConfined` was created.
    ///
    /// If this fails, the value will be leaked immediately.
    pub(crate) fn extract(mut self) -> Option<T> {
        if self.is_original_thread() {
            self.value.take()
        } else {
            None // causes Drop -> leak.
        }
    }

    fn is_original_thread(&self) -> bool {
        self.thread_id == std::thread::current().id()
    }
}

impl<T> Drop for ThreadConfined<T> {
    fn drop(&mut self) {
        if !self.is_original_thread() {
            std::mem::forget(self.value.take());

            // Cannot panic, potentially during unwind already.
            godot_error!(
                "Dropped ThreadConfined<T> on a different thread than it was created on. The inner T value will be leaked."
            );
        }
    }
}

unsafe impl<T: GodotClass> DynamicSend for ThreadConfined<Gd<T>> {
    type Inner = Gd<T>;

    fn extract_if_safe(self) -> Option<Self::Inner> {
        self.extract()
    }
}

impl<T: GodotClass> Sealed for ThreadConfined<Gd<T>> {}

impl<T: GodotClass> IntoDynamicSend for Gd<T> {
    type Target = ThreadConfined<Self>;

    fn into_dynamic_send(self) -> Self::Target {
        ThreadConfined::new(self)
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Generated impls

#[macro_export(local_inner_macros)]
macro_rules! impl_dynamic_send {
    (Send; $($ty:ty),+) => {
        $(
            unsafe impl $crate::task::DynamicSend for $ty {
                type Inner = Self;

                fn extract_if_safe(self) -> Option<Self::Inner> {
                    Some(self)
                }
            }

            impl $crate::task::IntoDynamicSend for $ty {
                type Target = Self;
                fn into_dynamic_send(self) -> Self::Target {
                    self
                }
            }
        )+
    };

    (tuple; $($arg:ident: $ty:ident),*) => {
        unsafe impl<$($ty: $crate::task::DynamicSend ),*> $crate::task::DynamicSend for ($($ty,)*) {
            type Inner = ($($ty::Inner,)*);

            fn extract_if_safe(self) -> Option<Self::Inner> {
                #[allow(non_snake_case)]
                let ($($arg,)*) = self;

                #[allow(clippy::unused_unit)]
                match ($($arg.extract_if_safe(),)*) {
                    ($(Some($arg),)*) => Some(($($arg,)*)),

                    #[allow(unreachable_patterns)]
                    _ => None,
                }
            }
        }

        impl<$($ty: $crate::task::IntoDynamicSend),*> $crate::task::IntoDynamicSend for ($($ty,)*) {
            type Target = ($($ty::Target,)*);

            fn into_dynamic_send(self) -> Self::Target {
                #[allow(non_snake_case)]
                let ($($arg,)*) = self;

                #[allow(clippy::unused_unit)]
                ($($arg.into_dynamic_send(),)*)
            }
        }
    };

    (!Send; $($ty:ident),+) => {
        $(
            impl $crate::meta::sealed::Sealed for $crate::task::ThreadConfined<$crate::builtin::$ty> {}

            unsafe impl $crate::task::DynamicSend for $crate::task::ThreadConfined<$crate::builtin::$ty> {
                type Inner = $crate::builtin::$ty;

                fn extract_if_safe(self) -> Option<Self::Inner> {
                    self.extract()
                }
            }

            impl $crate::task::IntoDynamicSend for $crate::builtin::$ty {
                type Target = $crate::task::ThreadConfined<$crate::builtin::$ty>;

                fn into_dynamic_send(self) -> Self::Target {
                    $crate::task::ThreadConfined::new(self)
                }
            }
        )+
    };
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::thread;

    use super::{SignalFutureResolver, ThreadConfined};
    use crate::classes::Object;
    use crate::obj::Gd;
    use crate::sys;

    /// Test that the hash of a cloned future resolver is equal to its original version. With this equality in place, we can create new
    /// Callables that are equal to their original version but have separate reference counting.
    #[test]
    fn future_resolver_cloned_hash() {
        let resolver_a = SignalFutureResolver::<(Gd<Object>, i64)>::new(Arc::default());
        let resolver_b = resolver_a.clone();

        let hash_a = sys::hash_value(&resolver_a);
        let hash_b = sys::hash_value(&resolver_b);

        assert_eq!(hash_a, hash_b);
    }

    // Test that dropping ThreadConfined<T> on another thread leaks the inner value.
    #[test]
    fn thread_confined_extract() {
        let confined = ThreadConfined::new(772);
        assert_eq!(confined.extract(), Some(772));

        let confined = ThreadConfined::new(772);

        let handle = thread::spawn(move || {
            assert!(confined.extract().is_none());
        });
        handle.join().unwrap();
    }

    #[test]
    fn thread_confined_leak_on_other_thread() {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);

        struct DropCounter;
        impl Drop for DropCounter {
            fn drop(&mut self) {
                COUNTER.fetch_add(1, Ordering::SeqCst);
            }
        }

        let drop_counter = DropCounter;
        let confined = ThreadConfined::new(drop_counter);

        let handle = thread::spawn(move || drop(confined));
        handle.join().unwrap();

        // The counter should still be 0, meaning Drop was not called (leaked).
        assert_eq!(COUNTER.load(Ordering::SeqCst), 0);
    }
}
