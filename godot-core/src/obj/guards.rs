/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::fmt::Debug;
use std::ops::{Deref, DerefMut};

#[cfg(feature = "experimental-threads")]
use godot_cell::blocking::{InaccessibleGuard, MutGuard, RefGuard};
#[cfg(not(feature = "experimental-threads"))]
use godot_cell::panicking::{InaccessibleGuard, MutGuard, RefGuard};
use godot_ffi::out;

use crate::obj::script::ScriptInstance;
use crate::obj::{AsDyn, Gd, GodotClass, PassiveGd};

/// Immutably/shared bound reference guard for a [`Gd`][crate::obj::Gd] smart pointer.
///
/// See [`Gd::bind`][crate::obj::Gd::bind] for usage.
// GdRef could technically implement Clone, but it wasn't needed so far.
#[derive(Debug)]
pub struct GdRef<'a, T: GodotClass> {
    guard: RefGuard<'a, T>,
}

impl<'a, T: GodotClass> GdRef<'a, T> {
    pub(crate) fn from_guard(guard: RefGuard<'a, T>) -> Self {
        Self { guard }
    }
}

impl<T: GodotClass> Deref for GdRef<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.guard
    }
}

impl<T: GodotClass> Drop for GdRef<'_, T> {
    fn drop(&mut self) {
        out!("GdRef drop: {:?}", std::any::type_name::<T>());
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Mutably/exclusively bound reference guard for a [`Gd`][crate::obj::Gd] smart pointer.
///
/// See [`Gd::bind_mut`][crate::obj::Gd::bind_mut] for usage.
#[derive(Debug)]
pub struct GdMut<'a, T: GodotClass> {
    guard: MutGuard<'a, T>,
}

impl<'a, T: GodotClass> GdMut<'a, T> {
    pub(crate) fn from_guard(guard: MutGuard<'a, T>) -> Self {
        Self { guard }
    }
}

impl<T: GodotClass> Deref for GdMut<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.guard
    }
}

impl<T: GodotClass> DerefMut for GdMut<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.guard
    }
}

impl<T: GodotClass> Drop for GdMut<'_, T> {
    fn drop(&mut self) {
        out!("GdMut drop: {:?}", std::any::type_name::<T>());
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Type-erased Gd guards

trait ErasedGuard<'a>: 'a {}

impl<'a, T: GodotClass> ErasedGuard<'a> for GdRef<'a, T> {}
impl<'a, T: GodotClass> ErasedGuard<'a> for GdMut<'a, T> {}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Shared reference guard for a [`DynGd`][crate::obj::DynGd] smart pointer.
///
/// Returned by [`DynGd::dyn_bind()`][crate::obj::DynGd::dyn_bind].
pub struct DynGdRef<'a, D: ?Sized> {
    /// Never accessed, but is kept alive to ensure dynamic borrow checks are upheld and the object isn't freed.
    _guard: Box<dyn ErasedGuard<'a>>,
    cached_ptr: *const D,
}

impl<'a, D> DynGdRef<'a, D>
where
    D: ?Sized + 'static,
{
    #[doc(hidden)]
    pub fn from_guard<T: AsDyn<D>>(guard: GdRef<'a, T>) -> Self {
        let obj = &*guard;
        let dyn_obj = obj.dyn_upcast();

        // Note: this pointer is persisted because it is protected by the guard, and the original T instance is pinned during that.
        // Caching prevents extra indirections; any calls through the dyn guard after the first is simply a Rust dyn-trait virtual call.
        let cached_ptr = std::ptr::addr_of!(*dyn_obj);

        Self {
            _guard: Box::new(guard),
            cached_ptr,
        }
    }
}

impl<D: ?Sized> Deref for DynGdRef<'_, D> {
    type Target = D;

    fn deref(&self) -> &D {
        // SAFETY: pointer refers to object that is pinned while guard is alive.
        unsafe { &*self.cached_ptr }
    }
}

impl<D: ?Sized> Drop for DynGdRef<'_, D> {
    fn drop(&mut self) {
        out!("DynGdRef drop: {:?}", std::any::type_name::<D>());
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Mutably/exclusively bound reference guard for a [`DynGd`][crate::obj::DynGd] smart pointer.
///
/// Returned by [`DynGd::dyn_bind_mut()`][crate::obj::DynGd::dyn_bind_mut].
pub struct DynGdMut<'a, D: ?Sized> {
    /// Never accessed, but is kept alive to ensure dynamic borrow checks are upheld and the object isn't freed.
    _guard: Box<dyn ErasedGuard<'a>>,
    cached_ptr: *mut D,
}

impl<'a, D> DynGdMut<'a, D>
where
    D: ?Sized + 'static,
{
    #[doc(hidden)]
    pub fn from_guard<T: AsDyn<D>>(mut guard: GdMut<'a, T>) -> Self {
        let obj = &mut *guard;
        let dyn_obj = obj.dyn_upcast_mut();

        // Note: this pointer is persisted because it is protected by the guard, and the original T instance is pinned during that.
        // Caching prevents extra indirections; any calls through the dyn guard after the first is simply a Rust dyn-trait virtual call.
        let cached_ptr = std::ptr::addr_of_mut!(*dyn_obj);

        Self {
            _guard: Box::new(guard),
            cached_ptr,
        }
    }
}

impl<D: ?Sized> Deref for DynGdMut<'_, D> {
    type Target = D;

    fn deref(&self) -> &D {
        // SAFETY: pointer refers to object that is pinned while guard is alive.
        unsafe { &*self.cached_ptr }
    }
}

impl<D: ?Sized> DerefMut for DynGdMut<'_, D> {
    fn deref_mut(&mut self) -> &mut D {
        // SAFETY: pointer refers to object that is pinned while guard is alive.
        unsafe { &mut *self.cached_ptr }
    }
}

impl<D: ?Sized> Drop for DynGdMut<'_, D> {
    fn drop(&mut self) {
        out!("DynGdMut drop: {:?}", std::any::type_name::<D>());
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

macro_rules! make_base_ref {
    ($ident:ident, $bound:ident, $doc_type:ident, $doc_path:path, $object_name:literal) => {
        /// Shared reference guard for a [`Base`](crate::obj::Base) pointer.
        ///
        #[doc = concat!("This can be used to call methods on the base object of a ", $object_name, " that takes `&self` as the receiver.\n\n")]
        #[doc = concat!("See [`", stringify!($doc_type), "::base()`](", stringify!($doc_path), "::base()) for usage.")]
        pub struct $ident<'a, T: $bound> {
            passive_gd: PassiveGd<T::Base>,
            _instance: &'a T,
        }

        impl<'a, T: $bound> $ident<'a, T> {
            pub(crate) fn new(passive_gd: PassiveGd<T::Base>, instance: &'a T) -> Self {
                Self {
                    passive_gd,
                    _instance: instance,
                }
            }
        }

        impl<T: $bound> Deref for $ident<'_, T> {
            type Target = Gd<T::Base>;

            fn deref(&self) -> &Gd<T::Base> {
                &self.passive_gd
            }
        }
    };
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

macro_rules! make_base_mut {
    ($ident:ident, $bound:ident, $doc_type:ident, $doc_path:path, $object_name:literal) => {
        /// Mutable/exclusive reference guard for a [`Base`](crate::obj::Base) pointer.
        ///
        /// This can be used to call methods on the base object of a Rust object, which takes `&self` or `&mut self` as the receiver.
        ///
        #[doc = concat!("See [`", stringify!($doc_type), "::base_mut()`](", stringify!($doc_path), "::base_mut()) for usage.\n")]
        pub struct $ident<'a, T: $bound> {
            passive_gd: PassiveGd<T::Base>,
            _inaccessible_guard: InaccessibleGuard<'a, T>,
        }

        impl<'a, T: $bound> $ident<'a, T> {
            pub(crate) fn new(
                passive_gd: PassiveGd<T::Base>,
                inaccessible_guard: InaccessibleGuard<'a, T>,
            ) -> Self {
                Self {
                    passive_gd,
                    _inaccessible_guard: inaccessible_guard,
                }
            }
        }

        impl<T: $bound> Deref for $ident<'_, T> {
            type Target = Gd<T::Base>;

            fn deref(&self) -> &Gd<T::Base> {
                &self.passive_gd
            }
        }

        impl<T: $bound> DerefMut for $ident<'_, T> {
            fn deref_mut(&mut self) -> &mut Gd<T::Base> {
                &mut self.passive_gd
            }
        }
    };
}

make_base_ref!(
    BaseRef,
    GodotClass,
    WithBaseField,
    super::WithBaseField,
    "rust object"
);
make_base_mut!(
    BaseMut,
    GodotClass,
    WithBaseField,
    super::WithBaseField,
    "rust object"
);

make_base_ref!(
    ScriptBaseRef,
    ScriptInstance,
    SiMut,
    crate::obj::script::SiMut,
    "[`ScriptInstance`]"
);
make_base_mut!(
    ScriptBaseMut,
    ScriptInstance,
    SiMut,
    crate::obj::script::SiMut,
    "['ScriptInstance']"
);
