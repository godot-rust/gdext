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

use crate::builtin::{Callable, RustCallable, Signal, Variant};
use crate::classes::object::ConnectFlags;
use crate::meta::ParamTuple;
use crate::obj::{EngineBitfield, WithBaseField};
use crate::registry::signal::TypedSignal;

/// The panicking counter part to the [`FallibleSignalFuture`].
///
/// This future works in the same way as `FallibleSignalFuture`, but panics when the signal object is freed, instead of resolving to a
/// [`Result::Err`].
pub struct SignalFuture<R: ParamTuple + Sync + Send>(FallibleSignalFuture<R>);

impl<R: ParamTuple + Sync + Send> SignalFuture<R> {
    fn new(signal: Signal) -> Self {
        Self(FallibleSignalFuture::new(signal))
    }
}

impl<R: ParamTuple + Sync + Send> Future for SignalFuture<R> {
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
#[cfg_attr(feature = "trace", derive(Default))]
pub struct SignalFutureResolver<R> {
    data: Arc<Mutex<SignalFutureData<R>>>,
}

impl<R> Clone for SignalFutureResolver<R> {
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
        }
    }
}

impl<R> SignalFutureResolver<R> {
    fn new(data: Arc<Mutex<SignalFutureData<R>>>) -> Self {
        Self { data }
    }
}

impl<R> std::hash::Hash for SignalFutureResolver<R> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_usize(Arc::as_ptr(&self.data) as usize);
    }
}

impl<R> PartialEq for SignalFutureResolver<R> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.data, &other.data)
    }
}

impl<R: ParamTuple + Sync + Send> RustCallable for SignalFutureResolver<R> {
    fn invoke(&mut self, args: &[&Variant]) -> Result<Variant, ()> {
        let waker = {
            let mut data = self.data.lock().unwrap();
            data.state = SignalFutureState::Ready(R::from_variant_array(args));

            // We no longer need the waker after we resolved. If the future is polled again, we'll also get a new waker.
            data.waker.take()
        };

        if let Some(waker) = waker {
            waker.wake();
        }

        Ok(Variant::nil())
    }
}

impl<R> Display for SignalFutureResolver<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SignalFutureResolver::<{}>", std::any::type_name::<R>())
    }
}

// This resolver will change the futures state when it's being dropped (i.e. the engine removes all connected signal callables). By marking
// the future as dead we can resolve it to an error value the next time it gets polled.
impl<R> Drop for SignalFutureResolver<R> {
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
pub struct FallibleSignalFuture<R: ParamTuple + Sync + Send> {
    data: Arc<Mutex<SignalFutureData<R>>>,
    callable: SignalFutureResolver<R>,
    signal: Signal,
}

impl<R: ParamTuple + Sync + Send> FallibleSignalFuture<R> {
    fn new(signal: Signal) -> Self {
        debug_assert!(
            !signal.is_null(),
            "Failed to create a future for an invalid Signal!\nEither the signal object was already freed or the signal was not registered in the object before using it.",
        );

        let data = Arc::new(Mutex::new(SignalFutureData::default()));

        // The callable currently requires that the return value is Sync + Send.
        let callable = SignalFutureResolver::new(data.clone());

        signal.connect(
            &Callable::from_custom(callable.clone()),
            ConnectFlags::ONE_SHOT.ord() as i64,
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

        match value {
            SignalFutureState::Pending => Poll::Pending,
            SignalFutureState::Dropped => unreachable!(),
            SignalFutureState::Dead => Poll::Ready(Err(FallibleSignalFutureError)),
            SignalFutureState::Ready(value) => Poll::Ready(Ok(value)),
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

impl<R: ParamTuple + Sync + Send> Future for FallibleSignalFuture<R> {
    type Output = Result<R, FallibleSignalFutureError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.get_mut().poll(cx)
    }
}

impl<R: ParamTuple + Sync + Send> Drop for FallibleSignalFuture<R> {
    fn drop(&mut self) {
        // The callable might alredy be destroyed, this occurs during engine shutdown.
        if self.signal.object().is_none() {
            return;
        }

        let mut data_lock = self.data.lock().unwrap();

        data_lock.state = SignalFutureState::Dropped;

        drop(data_lock);

        // We create a new Godot Callable from our RustCallable so we get independent reference counting.
        let gd_callable = Callable::from_custom(self.callable.clone());

        // is_connected will return true if the signal was never emited before the future is dropped.
        if self.signal.is_connected(&gd_callable) {
            self.signal.disconnect(&gd_callable);
        }
    }
}

impl Signal {
    /// Creates a fallible future for this signal.
    ///
    /// The future will resolve the next time the signal is emitted.
    /// See [`TrySignalFuture`] for details.
    ///
    /// Since the `Signal` type does not contain information on the signal argument types, the future output type has to be inferred from
    /// the call to this function.
    pub fn to_fallible_future<R: ParamTuple + Sync + Send>(&self) -> FallibleSignalFuture<R> {
        FallibleSignalFuture::new(self.clone())
    }

    /// Creates a future for this signal.
    ///
    /// The future will resolve the next time the signal is emitted, but might panic if the signal object is freed.
    /// See [`SignalFuture`] for details.
    ///
    /// Since the `Signal` type does not contain information on the signal argument types, the future output type has to be inferred from
    /// the call to this function.
    pub fn to_future<R: ParamTuple + Sync + Send>(&self) -> SignalFuture<R> {
        SignalFuture::new(self.clone())
    }
}

impl<C: WithBaseField, R: ParamTuple + Sync + Send> TypedSignal<'_, C, R> {
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

impl<C: WithBaseField, R: ParamTuple + Sync + Send> IntoFuture for &TypedSignal<'_, C, R> {
    type Output = R;

    type IntoFuture = SignalFuture<R>;

    fn into_future(self) -> Self::IntoFuture {
        self.to_future()
    }
}

#[cfg(test)]
mod tests {
    use crate::sys;
    use std::sync::Arc;

    use super::SignalFutureResolver;

    /// Test that the hash of a cloned future resolver is equal to its original version. With this equality in place, we can create new
    /// Callables that are equal to their original version but have separate reference counting.
    #[test]
    fn future_resolver_cloned_hash() {
        let resolver_a = SignalFutureResolver::<u8>::new(Arc::default());
        let resolver_b = resolver_a.clone();

        let hash_a = sys::hash_value(&resolver_a);
        let hash_b = sys::hash_value(&resolver_b);

        assert_eq!(hash_a, hash_b);
    }
}
