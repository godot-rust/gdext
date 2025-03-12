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
use crate::meta::sealed::Sealed;
use crate::meta::ParamTuple;
use crate::obj::{EngineBitfield, Gd, GodotClass, WithBaseField};
use crate::registry::signal::TypedSignal;

/// The panicking counter part to the [`FallibleSignalFuture`].
///
/// This future works in the same way as `FallibleSignalFuture`, but panics when the signal object is freed, instead of resolving to a
/// [`Result::Err`].
///
/// # Panics
/// - If the signal object is freed before the signal has been emitted.
/// - If one of the signal arguments is `!Send`, but the signal was emitted on a different thread.
pub struct SignalFuture<R: ParamTuple + IntoMaybeSend>(FallibleSignalFuture<R>);

impl<R: ParamTuple + IntoMaybeSend> SignalFuture<R> {
    fn new(signal: Signal) -> Self {
        Self(FallibleSignalFuture::new(signal))
    }
}

impl<R: ParamTuple + IntoMaybeSend> Future for SignalFuture<R> {
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
pub struct SignalFutureResolver<R: IntoMaybeSend> {
    data: Arc<Mutex<SignalFutureData<R::Target>>>,
}

impl<R: IntoMaybeSend> Clone for SignalFutureResolver<R> {
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
        }
    }
}

// Implement Default for itest if R is also Default.
#[cfg(feature = "trace")]
impl<R: IntoMaybeSend + Default> Default for SignalFutureResolver<R> {
    fn default() -> Self {
        Self {
            data: Default::default(),
        }
    }
}

impl<R: IntoMaybeSend> SignalFutureResolver<R> {
    fn new(data: Arc<Mutex<SignalFutureData<R::Target>>>) -> Self {
        Self { data }
    }
}

impl<R: IntoMaybeSend> std::hash::Hash for SignalFutureResolver<R> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_usize(Arc::as_ptr(&self.data) as usize);
    }
}

impl<R: IntoMaybeSend> PartialEq for SignalFutureResolver<R> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.data, &other.data)
    }
}

impl<R: ParamTuple + IntoMaybeSend> RustCallable for SignalFutureResolver<R> {
    fn invoke(&mut self, args: &[&Variant]) -> Result<Variant, ()> {
        let waker = {
            let mut data = self.data.lock().unwrap();
            data.state = SignalFutureState::Ready(R::from_variant_array(args).into_maybe_send());

            // We no longer need the waker after we resolved. If the future is polled again, we'll also get a new waker.
            data.waker.take()
        };

        if let Some(waker) = waker {
            waker.wake();
        }

        Ok(Variant::nil())
    }
}

impl<R: IntoMaybeSend> Display for SignalFutureResolver<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SignalFutureResolver::<{}>", std::any::type_name::<R>())
    }
}

// This resolver will change the futures state when it's being dropped (i.e. the engine removes all connected signal callables). By marking
// the future as dead we can resolve it to an error value the next time it gets polled.
impl<R: IntoMaybeSend> Drop for SignalFutureResolver<R> {
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
pub struct FallibleSignalFuture<R: ParamTuple + IntoMaybeSend> {
    data: Arc<Mutex<SignalFutureData<R::Target>>>,
    callable: SignalFutureResolver<R>,
    signal: Signal,
}

impl<R: ParamTuple + IntoMaybeSend> FallibleSignalFuture<R> {
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
            SignalFutureState::Ready(value) => {
                let Some(value) = AssertSafeSend::assert_safe_send(value) else {
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

impl<R: ParamTuple + IntoMaybeSend> Future for FallibleSignalFuture<R> {
    type Output = Result<R, FallibleSignalFutureError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.get_mut().poll(cx)
    }
}

impl<R: ParamTuple + IntoMaybeSend> Drop for FallibleSignalFuture<R> {
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
    pub fn to_fallible_future<R: ParamTuple + IntoMaybeSend>(&self) -> FallibleSignalFuture<R> {
        FallibleSignalFuture::new(self.clone())
    }

    /// Creates a future for this signal.
    ///
    /// The future will resolve the next time the signal is emitted, but might panic if the signal object is freed.
    /// See [`SignalFuture`] for details.
    ///
    /// Since the `Signal` type does not contain information on the signal argument types, the future output type has to be inferred from
    /// the call to this function.
    pub fn to_future<R: ParamTuple + IntoMaybeSend>(&self) -> SignalFuture<R> {
        SignalFuture::new(self.clone())
    }
}

impl<C: WithBaseField, R: ParamTuple + IntoMaybeSend> TypedSignal<'_, C, R> {
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

impl<C: WithBaseField, R: ParamTuple + IntoMaybeSend> IntoFuture for &TypedSignal<'_, C, R> {
    type Output = R;

    type IntoFuture = SignalFuture<R>;

    fn into_future(self) -> Self::IntoFuture {
        self.to_future()
    }
}

/// Convert a value into a type that is [`Send`] at compile-time while the value might not be.
///
/// This allows to turn any implementor into a type that is `Send`, but requires to also implement [`AssertSafeSend`] as well.
/// The later trait will verify if a value can actually be sent between threads at runtime.
pub trait IntoMaybeSend: Sealed {
    type Target: Send + AssertSafeSend<Target = Self>;

    fn into_maybe_send(self) -> Self::Target;
}

/// Assert that it was safe to send the value to the current thread.
///
/// # Safety
/// The implementor has to guarantee that `assert_safe_send` panics if the value has been sent between threads while being `!Send`.
pub unsafe trait AssertSafeSend: Send + Sealed {
    type Target;

    fn assert_safe_send(self) -> Option<Self::Target>;
}

pub struct ThreadConfined<T> {
    value: T,
    thread_id: ThreadId,
}

unsafe impl<T> Send for ThreadConfined<T> {}
unsafe impl<T> Sync for ThreadConfined<T> {}

impl<T> ThreadConfined<T> {
    fn new(value: T) -> Self {
        Self {
            value,
            thread_id: std::thread::current().id(),
        }
    }

    fn extract(self) -> Option<T> {
        (self.thread_id == std::thread::current().id()).then_some(self.value)
    }
}

unsafe impl<T: GodotClass> AssertSafeSend for ThreadConfined<Gd<T>> {
    type Target = Gd<T>;

    fn assert_safe_send(self) -> Option<Self::Target> {
        self.extract()
    }
}

impl<T: GodotClass> Sealed for ThreadConfined<Gd<T>> {}

impl<T: GodotClass> IntoMaybeSend for Gd<T> {
    type Target = ThreadConfined<Self>;

    fn into_maybe_send(self) -> Self::Target {
        ThreadConfined::new(self)
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Generated impls

macro_rules! impl_maybe_send {
    (Send; $($ty:ty),+) => {
        $(
            unsafe impl AssertSafeSend for $ty {
                type Target = Self;

                fn assert_safe_send(self) -> Option<Self::Target> {
                    Some(self)
                }
            }

            impl IntoMaybeSend for $ty {
                type Target = Self;
                fn into_maybe_send(self) -> Self::Target {
                    self
                }
            }
        )+
    };

    (Send; builtin::{$($ty:ident),+}) => {
        impl_maybe_send!(Send; $($crate::builtin::$ty),+);
    };

    (tuple; $($arg:ident: $ty:ident),*) => {
        unsafe impl<$($ty: AssertSafeSend),*> AssertSafeSend for ($($ty,)*) {
            type Target = ($($ty::Target,)*);

            fn assert_safe_send(self) -> Option<Self::Target> {
                #[allow(non_snake_case)]
                let ($($arg,)*) = self;

                #[allow(clippy::unused_unit)]
                match ($($arg.assert_safe_send(),)*) {
                    ($(Some($arg),)*) => Some(($($arg,)*)),

                    #[allow(unreachable_patterns)]
                    _ => None,
                }
            }
        }

        impl<$($ty: IntoMaybeSend),*> IntoMaybeSend for ($($ty,)*) {
            type Target = ($($ty::Target,)*);

            fn into_maybe_send(self) -> Self::Target {
                #[allow(non_snake_case)]
                let ($($arg,)*) = self;

                #[allow(clippy::unused_unit)]
                ($($arg.into_maybe_send(),)*)
            }
        }
    };

    (!Send; $($ty:ident),+) => {
        $(
            impl Sealed for ThreadConfined<$crate::builtin::$ty> {}

            unsafe impl AssertSafeSend for ThreadConfined<$crate::builtin::$ty> {
                type Target = $crate::builtin::$ty;

                fn assert_safe_send(self) -> Option<Self::Target> {
                    self.extract()
                }
            }

            impl IntoMaybeSend for $crate::builtin::$ty {
                type Target = ThreadConfined<$crate::builtin::$ty>;

                fn into_maybe_send(self) -> Self::Target {
                    ThreadConfined::new(self)
                }
            }
        )+
    };
}

impl_maybe_send!(
    Send;
    bool, u8, u16, u32, u64, i8, i16, i32, i64, f32, f64
);

impl_maybe_send!(
    Send;
    builtin::{
        StringName, Transform2D, Transform3D, Vector2, Vector2i, Vector2Axis,
        Vector3, Vector3i, Vector3Axis, Vector4, Vector4i, Rect2, Rect2i, Plane, Quaternion, Aabb, Basis, Projection, Color, Rid
    }
);

impl_maybe_send!(
    !Send;
    Variant, GString, Dictionary, VariantArray, Callable, NodePath, PackedByteArray, PackedInt32Array, PackedInt64Array, PackedFloat32Array,
    PackedFloat64Array, PackedStringArray, PackedVector2Array, PackedVector3Array, PackedColorArray, Signal
);

#[cfg(since_api = "4.3")]
impl_maybe_send!(!Send; PackedVector4Array);

// This should be kept in sync with crate::registry::signal::variadic.
impl_maybe_send!(tuple; );
impl_maybe_send!(tuple; arg1: A1);
impl_maybe_send!(tuple; arg1: A1, arg2: A2);
impl_maybe_send!(tuple; arg1: A1, arg2: A2, arg3: A3);
impl_maybe_send!(tuple; arg1: A1, arg2: A2, arg3: A3, arg4: A4);
impl_maybe_send!(tuple; arg1: A1, arg2: A2, arg3: A3, arg4: A4, arg5: A5);
impl_maybe_send!(tuple; arg1: A1, arg2: A2, arg3: A3, arg4: A4, arg5: A5, arg6: A6);
impl_maybe_send!(tuple; arg1: A1, arg2: A2, arg3: A3, arg4: A4, arg5: A5, arg6: A6, arg7: A7);
impl_maybe_send!(tuple; arg1: A1, arg2: A2, arg3: A3, arg4: A4, arg5: A5, arg6: A6, arg7: A7, arg8: A8);
impl_maybe_send!(tuple; arg1: A1, arg2: A2, arg3: A3, arg4: A4, arg5: A5, arg6: A6, arg7: A7, arg8: A8, arg9: A9);

#[cfg(test)]
mod tests {
    use godot_ffi::VariantType;

    use crate::builtin::{
        Aabb, Basis, Callable, Color, Dictionary, GString, NodePath, PackedByteArray,
        PackedColorArray, PackedFloat32Array, PackedFloat64Array, PackedInt32Array,
        PackedInt64Array, PackedStringArray, PackedVector2Array, PackedVector3Array,
        PackedVector4Array, Plane, Projection, Quaternion, Rect2, Rect2i, Rid, Signal, StringName,
        Transform2D, Transform3D, Variant, VariantArray, Vector2, Vector2i, Vector3, Vector3i,
        Vector4, Vector4i,
    };
    use crate::classes::Object;
    use crate::meta::GodotType;
    use crate::obj::{Gd, IndexEnum};
    use crate::sys;
    use std::sync::Arc;

    use super::SignalFutureResolver;

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

    // Compile time check that we cover all the Variant types with IntoMaybeSend and AssertSafeSend impls.
    const _: () = {
        const fn variant_type<T: super::IntoMaybeSend + GodotType>() -> VariantType {
            <T::Ffi as sys::GodotFfi>::VARIANT_TYPE
        }

        const NIL: VariantType = variant_type::<Variant>();
        const BOOL: VariantType = variant_type::<bool>();
        const I64: VariantType = variant_type::<i64>();
        const F64: VariantType = variant_type::<f64>();
        const GSTRING: VariantType = variant_type::<GString>();

        const VECTOR2: VariantType = variant_type::<Vector2>();
        const VECTOR2I: VariantType = variant_type::<Vector2i>();
        const RECT2: VariantType = variant_type::<Rect2>();
        const RECT2I: VariantType = variant_type::<Rect2i>();
        const VECTOR3: VariantType = variant_type::<Vector3>();
        const VECTOR3I: VariantType = variant_type::<Vector3i>();
        const TRANSFORM2D: VariantType = variant_type::<Transform2D>();
        const TRANSFORM3D: VariantType = variant_type::<Transform3D>();
        const VECTOR4: VariantType = variant_type::<Vector4>();
        const VECTOR4I: VariantType = variant_type::<Vector4i>();
        const PLANE: VariantType = variant_type::<Plane>();
        const QUATERNION: VariantType = variant_type::<Quaternion>();
        const AABB: VariantType = variant_type::<Aabb>();
        const BASIS: VariantType = variant_type::<Basis>();
        const PROJECTION: VariantType = variant_type::<Projection>();
        const COLOR: VariantType = variant_type::<Color>();
        const STRING_NAME: VariantType = variant_type::<StringName>();
        const NODE_PATH: VariantType = variant_type::<NodePath>();
        const RID: VariantType = variant_type::<Rid>();
        const OBJECT: VariantType = variant_type::<Gd<Object>>();
        const CALLABLE: VariantType = variant_type::<Callable>();
        const SIGNAL: VariantType = variant_type::<Signal>();
        const DICTIONARY: VariantType = variant_type::<Dictionary>();
        const ARRAY: VariantType = variant_type::<VariantArray>();
        const PACKED_BYTE_ARRAY: VariantType = variant_type::<PackedByteArray>();
        const PACKED_INT32_ARRAY: VariantType = variant_type::<PackedInt32Array>();
        const PACKED_INT64_ARRAY: VariantType = variant_type::<PackedInt64Array>();
        const PACKED_FLOAT32_ARRAY: VariantType = variant_type::<PackedFloat32Array>();
        const PACKED_FLOAT64_ARRAY: VariantType = variant_type::<PackedFloat64Array>();
        const PACKED_STRING_ARRAY: VariantType = variant_type::<PackedStringArray>();
        const PACKED_VECTOR2_ARRAY: VariantType = variant_type::<PackedVector2Array>();
        const PACKED_VECTOR3_ARRAY: VariantType = variant_type::<PackedVector3Array>();
        const PACKED_COLOR_ARRAY: VariantType = variant_type::<PackedColorArray>();
        const PACKED_VECTOR4_ARRAY: VariantType = variant_type::<PackedVector4Array>();

        const MAX: i32 = VariantType::ENUMERATOR_COUNT as i32;

        // The matched value is not relevant, we just want to ensure that the full list from 0 to MAX is covered.
        match VariantType::STRING {
            VariantType { ord: i32::MIN..0 } => panic!("ord is out of defined range!"),
            NIL => (),
            BOOL => (),
            I64 => (),
            F64 => (),
            GSTRING => (),
            VECTOR2 => (),
            VECTOR2I => (),
            RECT2 => (),
            RECT2I => (),
            VECTOR3 => (),
            VECTOR3I => (),
            TRANSFORM2D => (),
            VECTOR4 => (),
            VECTOR4I => (),
            PLANE => (),
            QUATERNION => (),
            AABB => (),
            BASIS => (),
            TRANSFORM3D => (),
            PROJECTION => (),
            COLOR => (),
            STRING_NAME => (),
            NODE_PATH => (),
            RID => (),
            OBJECT => (),
            CALLABLE => (),
            SIGNAL => (),
            DICTIONARY => (),
            ARRAY => (),
            PACKED_BYTE_ARRAY => (),
            PACKED_INT32_ARRAY => (),
            PACKED_INT64_ARRAY => (),
            PACKED_FLOAT32_ARRAY => (),
            PACKED_FLOAT64_ARRAY => (),
            PACKED_STRING_ARRAY => (),
            PACKED_VECTOR2_ARRAY => (),
            PACKED_VECTOR3_ARRAY => (),
            PACKED_COLOR_ARRAY => (),
            PACKED_VECTOR4_ARRAY => (),
            VariantType { ord: MAX.. } => panic!("ord is out of defined range!"),
        }
    };
}
