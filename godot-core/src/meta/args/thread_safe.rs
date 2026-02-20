/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::meta::{CowArg, GodotConvert, NullArg};
use crate::obj::{DynGd, Gd, GodotClass};

pub(crate) trait ThreadSafeSealed {}

/// Runtime restrictions for [`AsArg`](super::AsArg) to guarantee thread safe usage of engine types.
///
/// For thread safe types that implement `Send + Sync` this trait is implemented as a no-op. For non thread-safe engine types, a runtime
/// check is performed that ensures the types can only be passed to the engine on the main thread.
#[expect(private_bounds)]
pub trait ThreadSafeArgContext: ThreadSafeSealed {
    /// Panics if the value is being used in a non thread-safe context.
    fn guarantee_thread_safe();
}

/// Maker trait to implement [`ThreadSafeArgContext`] for thread-safe types.
///
/// Due to type system constraints `ThreadSafeArgContext` can not be implemented for all `Sync + Send` types. This marker trait
/// allows to implement the sealed trait for types that are thread-safe.
pub trait ThreadSafeArg: ThreadSafeArgContext + GodotConvert
where
    <Self as GodotConvert>::Via: Send + Sync,
{
}

impl<T: ThreadSafeArg> ThreadSafeArgContext for T
where
    <Self as GodotConvert>::Via: Send + Sync,
{
    fn guarantee_thread_safe() {}
}

impl<T: ThreadSafeArg> ThreadSafeSealed for T where <Self as GodotConvert>::Via: Send + Sync {}

#[macro_export(local_inner_macros)]
macro_rules! impl_thread_safe_arg {
    ([$($bounds:tt)*] $ty:ty) => {
        impl<$($bounds)*> $crate::meta::ThreadSafeArgContext for $ty {
            fn guarantee_thread_safe() {}
        }

        impl<$($bounds)*> $crate::meta::ThreadSafeSealed for $ty {}
    };

    ($($ty:ty),+) => {
        $(impl_thread_safe_arg!([] $ty);)+
    };
}

#[macro_export(local_inner_macros)]
macro_rules! impl_non_thread_safe_arg {
    ([$($bounds:tt)*] $ty:ty) => {
        impl<$($bounds)*> $crate::meta::ThreadSafeArgContext for $ty {
            fn guarantee_thread_safe() {
                if !$crate::sys::is_main_thread() {
                    ::std::panic!(
                        "Value of type {} can not be passed to the engine outside the main thread",
                        ::std::any::type_name::<$ty>()
                    );
                }
            }
        }

        impl<$($bounds)*> $crate::meta::ThreadSafeSealed for $ty {}
    };

    ($($ty:ty),+) => {
        $(impl_non_thread_safe_arg!([] $ty);)+
    };
}

impl<T: ThreadSafeArgContext> ThreadSafeArgContext for Option<T> {
    fn guarantee_thread_safe() {
        T::guarantee_thread_safe();
    }
}

impl<T: ThreadSafeSealed> ThreadSafeSealed for Option<T> {}

impl_thread_safe_arg!(String, &String, &str);
impl_non_thread_safe_arg!([T: GodotClass] Gd<T>);
impl_non_thread_safe_arg!([T: GodotClass] &Gd<T>);
impl_non_thread_safe_arg!([T: GodotClass, D: ?Sized + 'static] DynGd<T, D>);
impl_non_thread_safe_arg!([T: GodotClass, D: ?Sized] &DynGd<T, D>);

// NullArg<T> maps to None so it should be fine to be passed to the engine.
impl<T> ThreadSafeArgContext for NullArg<T> {
    fn guarantee_thread_safe() {}
}

impl<T> ThreadSafeSealed for NullArg<T> {}

impl<T> ThreadSafeArgContext for CowArg<'_, T>
where
    for<'a> T: ThreadSafeArgContext,
{
    fn guarantee_thread_safe() {
        T::guarantee_thread_safe();
    }
}

impl<T> ThreadSafeSealed for CowArg<'_, T> where for<'a> T: ThreadSafeSealed {}
