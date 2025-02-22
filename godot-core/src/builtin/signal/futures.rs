/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use core::panic;
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

/// The panicking counter part to the [`TrySignalFuture`].
///
/// This future works in the same ways as the TrySignalFuture, but panics when the signal object is freed instead of resolving to an
/// [`Result::Err`]
pub struct SignalFuture<R: FromSignalArgs>(TrySignalFuture<R>);

impl<R: FromSignalArgs> SignalFuture<R> {
    fn new(signal: Signal) -> Self {
        Self(TrySignalFuture::new(signal))
    }
}

impl<R: FromSignalArgs> Future for SignalFuture<R> {
    type Output = R;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let poll_result = self.get_mut().0.poll(cx);

        match poll_result {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Ok(value)) => Poll::Ready(value),
            Poll::Ready(Err(TrySignalFutureError)) => panic!(
                "The signal object of a SignalFuture was freed while the future was still waiting for the signal to be emitted!"
            ),
        }
    }
}

// Not derived, otherwise an extra bound `Output: Default` is required.
struct SignalFutureData<T> {
    state: TrySignalFutureState<T>,
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

impl<R: FromSignalArgs> RustCallable for SignalFutureResolver<R> {
    fn invoke(&mut self, args: &[&Variant]) -> Result<Variant, ()> {
        let mut data = self.data.lock().unwrap();
        // We no loger need the waker after we resolved. If the future get's polled again we also get a new waker.
        let waker = data.waker.take();

        data.state = TrySignalFutureState::Ready(R::from_args(args));
        drop(data);

        if let Some(waker) = waker {
            waker.wake();
        }

        Ok(Variant::nil())
    }
}

impl<R> Display for SignalFutureResolver<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "GuaranteedSignalFutureResolver::<{}>",
            std::any::type_name::<R>()
        )
    }
}

// this resolver will resolve the future when it's being dropped (i.e. the engine removes all connected signal callables). This is very unusual.
impl<R> Drop for SignalFutureResolver<R> {
    fn drop(&mut self) {
        let mut data = self.data.lock().unwrap();

        if !matches!(data.state, TrySignalFutureState::Pending) {
            return;
        }

        data.state = TrySignalFutureState::Dead;

        if let Some(ref waker) = data.waker {
            waker.wake_by_ref();
        }
    }
}

#[derive(Default)]
enum TrySignalFutureState<T> {
    #[default]
    Pending,
    Ready(T),
    Dead,
    Dropped,
}

impl<T> TrySignalFutureState<T> {
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
pub struct TrySignalFuture<R: FromSignalArgs> {
    data: Arc<Mutex<SignalFutureData<R>>>,
    callable: SignalFutureResolver<R>,
    signal: Signal,
}

impl<R: FromSignalArgs> TrySignalFuture<R> {
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
    fn poll(&mut self, cx: &mut Context<'_>) -> Poll<Result<R, TrySignalFutureError>> {
        let mut data = self.data.lock().unwrap();

        data.waker.replace(cx.waker().clone());

        let value = data.state.take();

        match value {
            TrySignalFutureState::Pending => Poll::Pending,
            TrySignalFutureState::Dropped => unreachable!(),
            TrySignalFutureState::Dead => Poll::Ready(Err(TrySignalFutureError)),
            TrySignalFutureState::Ready(value) => Poll::Ready(Ok(value)),
        }
    }
}

pub struct TrySignalFutureError;

impl<R: FromSignalArgs> Future for TrySignalFuture<R> {
    type Output = Result<R, TrySignalFutureError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.get_mut().poll(cx)
    }
}

impl<R: FromSignalArgs> Drop for TrySignalFuture<R> {
    fn drop(&mut self) {
        // The callable might alredy be destroyed, this occurs during engine shutdown.
        if self.signal.object().is_none() {
            return;
        }

        let mut data = self.data.lock().unwrap();

        data.state = TrySignalFutureState::Dropped;

        drop(data);

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
impl<R1: FromGodot + Sync + Send + 'static, R2: FromGodot + Sync + Send + 'static> FromSignalArgs
    for (R1, R2)
{
    fn from_args(args: &[&Variant]) -> Self {
        (args[0].to(), args[0].to())
    }
}

impl Signal {
    /// Creates a fallible future for this signal.
    ///
    /// The future will resolve the next time the signal is emitted.
    /// See [`TryFutureSignal`] for details.
    pub fn to_try_future<R: FromSignalArgs>(&self) -> TrySignalFuture<R> {
        TrySignalFuture::new(self.clone())
    }

    /// Creates a future for this `Signal`.
    ///
    /// The future will resolve the next time the `Signal` is emitted, but might panic if the signal object is freed.
    /// See [`FutureSignal`] for details.
    pub fn to_future<R: FromSignalArgs>(&self) -> SignalFuture<R> {
        SignalFuture::new(self.clone())
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
    fn guaranteed_future_resolver_cloned_hash() {
        let resolver_a = SignalFutureResolver::<u8>::new(Arc::default());
        let resolver_b = resolver_a.clone();

        let hash_a = sys::hash_value(&resolver_a);
        let hash_b = sys::hash_value(&resolver_b);

        assert_eq!(hash_a, hash_b);
    }
}
