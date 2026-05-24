/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot_ffi::VariantType;

use crate::builtin::Variant;
use crate::meta::sealed::Sealed;
use crate::meta::{CowArg, GodotConvert, NullArg, ToGodot};
use crate::obj::{DynGd, Gd, GodotClass};

pub(crate) trait ThreadSafeSealed {}

/// Runtime restrictions for [`AsArg`](super::AsArg) to guarantee thread safe usage of engine types.
///
/// For thread safe types that implement `Send + Sync` this trait is implemented as a no-op. For non thread-safe engine types, a runtime
/// check is performed that ensures the types can only be passed to the engine on the main thread.
#[expect(private_bounds)]
pub trait ThreadSafeArgContext: ThreadSafeSealed {
    /// Panics if the value is being used in a non thread-safe context.
    fn guarantee_thread_safe(&self);
}

impl<T: ToGodot<Threads = crate::meta::ThreadSafeArg> + Send> ThreadSafeArgContext for T
where
    <Self as GodotConvert>::Via: Send + Sync,
{
    fn guarantee_thread_safe(&self) {}
}

impl<T: ToGodot<Threads = crate::meta::ThreadSafeArg> + Send> ThreadSafeSealed for T where
    <Self as GodotConvert>::Via: Send + Sync
{
}

#[macro_export(local_inner_macros)]
macro_rules! impl_thread_safe_arg {
    ([$($bounds:tt)*] $ty:ty) => {
        impl<$($bounds)*> $crate::meta::ThreadSafeArgContext for $ty {
            fn guarantee_thread_safe(&self) {}
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
            fn guarantee_thread_safe(&self) {
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
    fn guarantee_thread_safe(&self) {
        if let Some(value) = self {
            value.guarantee_thread_safe();
        }
    }
}

impl<T: ThreadSafeSealed> ThreadSafeSealed for Option<T> {}

impl_non_thread_safe_arg!([T: GodotClass] Gd<T>);
impl_non_thread_safe_arg!([T: GodotClass] &Gd<T>);
impl_non_thread_safe_arg!([T: GodotClass, D: ?Sized + 'static] DynGd<T, D>);
impl_non_thread_safe_arg!([T: GodotClass, D: ?Sized] &DynGd<T, D>);
impl_thread_safe_arg!(&String);

// NullArg<T> maps to None so it should be fine to be passed to the engine.
impl<T> ThreadSafeArgContext for NullArg<T> {
    fn guarantee_thread_safe(&self) {}
}

impl<T> ThreadSafeSealed for NullArg<T> {}

impl<T> ThreadSafeArgContext for CowArg<'_, T>
where
    for<'a> T: ThreadSafeArgContext,
{
    fn guarantee_thread_safe(&self) {
        self.cow_as_ref().guarantee_thread_safe();
    }
}

impl<T> ThreadSafeSealed for CowArg<'_, T> where for<'a> T: ThreadSafeSealed {}

impl ThreadSafeArgContext for Variant {
    fn guarantee_thread_safe(&self) {
        match self.get_type() {
            VariantType::NIL
            | VariantType::BOOL
            | VariantType::INT
            | VariantType::FLOAT
            | VariantType::STRING
            | VariantType::VECTOR2
            | VariantType::VECTOR2I
            | VariantType::RECT2
            | VariantType::RECT2I
            | VariantType::VECTOR3
            | VariantType::VECTOR3I
            | VariantType::TRANSFORM2D
            | VariantType::VECTOR4
            | VariantType::VECTOR4I
            | VariantType::PLANE
            | VariantType::QUATERNION
            | VariantType::AABB
            | VariantType::BASIS
            | VariantType::TRANSFORM3D
            | VariantType::PROJECTION
            | VariantType::COLOR
            | VariantType::STRING_NAME
            | VariantType::RID => (),
            _ => {
                if !crate::sys::is_main_thread() {
                    ::std::panic!(
                        "Variant value of type {} can not be passed to the engine outside the main thread",
                        self.get_type().godot_type_name()
                    );
                }
            }
        }
    }
}

impl ThreadSafeArgContext for &Variant {
    fn guarantee_thread_safe(&self) {
        (*self).guarantee_thread_safe();
    }
}

impl ThreadSafeArgContext for &[Variant] {
    fn guarantee_thread_safe(&self) {
        for var in *self {
            var.guarantee_thread_safe();
        }
    }
}

impl ThreadSafeSealed for Variant {}
impl ThreadSafeSealed for &Variant {}
impl ThreadSafeSealed for &[Variant] {}

/// Determines if a type is expected to be thread safe or not.
///
/// See [ToGodot::Threads](crate::meta::ToGodot).
pub trait ThreadSafety: Sealed {}

/// Argument is thread-safe, the type has to be `Send`.
///
/// See [`ToGodot::Threads`].
pub struct ThreadSafeArg;

impl ThreadSafety for ThreadSafeArg {}
impl Sealed for ThreadSafeArg {}

/// Argument is not thread-safe, the type requires a custom implementation of [`ThreadSafeArgContext`].
///
/// See [`ToGodot::Threads`].
pub struct NonThreadSafeArg;

impl ThreadSafety for NonThreadSafeArg {}
impl Sealed for NonThreadSafeArg {}
