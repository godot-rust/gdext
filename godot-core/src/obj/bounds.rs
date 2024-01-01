/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Different ways how bounds of a `GodotClass` can be checked.
//!
//! This module contains three traits that can be used to check the characteristics of a `GodotClass` type:
//!
//! 1. [`Declarer`] tells you whether the class is provided by the engine or user-defined.
//!    - [`DeclEngine`] is used for all classes provided by the engine (e.g. `Node3D`).
//!    - [`DeclUser`] is used for all classes defined by the user, typically through `#[derive(GodotClass)]`.<br><br>
//!
//! 2. [`Memory`] is used to check the memory strategy of the **static** type.
//!
//!    This is useful when you operate on associated functions of `Gd<T>` or `T`, e.g. for construction.
//!    - [`MemRefCounted`] is used for `RefCounted` classes and derived.
//!    - [`MemManual`] is used for `Object` and all inherited classes, which are not `RefCounted` (e.g. `Node`).<br><br>
//!
//! 3. [`DynMemory`] is used to check the memory strategy of the **dynamic** type.
//!
//!    When you operate on methods of `T` or `Gd<T>` and are interested in instances, you can use this.
//!    Most of the time, this is not what you want -- just use `Memory` if you want to know if a type is manually managed or ref-counted.
//!    - [`MemRefCounted`] is used for `RefCounted` classes and derived. These are **always** reference-counted.
//!    - [`MemManual`] is used instances inheriting `Object`, which are not `RefCounted` (e.g. `Node`). Excludes `Object` itself. These are
//!      **always** manually managed.
//!    - [`MemDynamic`] is used for `Object` instances. `Gd<Object>` can point to objects of any possible class, so whether we are dealing with
//!      a ref-counted or manually-managed object is determined only at runtime.
//!
//!
//! # Example
//!
//! Declare a custom smart pointer which wraps `Gd<T>` pointers, but only accepts `T` objects that are manually managed.
//! ```
//! use godot::prelude::*;
//! use godot::obj::{bounds, Bounds};
//!
//! struct MyGd<T>
//! where T: GodotClass + Bounds<Memory = bounds::MemManual>
//! {
//!    inner: Gd<T>,
//! }
//! ```
//!
//! Note that depending on if you want to exclude `Object`, you should use `DynMemory` instead of `Memory`.

use crate::obj::cap::GodotDefault;
use crate::obj::{Bounds, Gd, GodotClass, RawGd};
use crate::storage::Storage;
use crate::{callbacks, out, sys};
use private::Sealed;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Sealed trait

pub(super) mod private {
    use super::{Declarer, DynMemory, Memory};

    // Bounds trait declared here for code locality; re-exported in crate::obj.

    /// Library-implemented trait to check bounds on `GodotClass` types.
    ///
    /// See also [`bounds`](crate::obj::bounds) module documentation.
    ///
    /// # Safety
    ///
    /// Internal.
    /// You **must not** implement this trait yourself. [`#[derive(GodotClass)`](../bind/derive.GodotClass.html) will automatically do it.
    pub unsafe trait Bounds {
        type Memory: Memory;

        /// Defines the memory strategy of the instance (at runtime).
        type DynMemory: DynMemory;

        /// Whether this class is a core Godot class provided by the engine, or declared by the user as a Rust struct.
        // TODO what about GDScript user classes?
        type Declarer: Declarer;
    }

    pub trait Sealed {}
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Memory bounds

/// Specifies the memory strategy of the static type.
pub trait Memory: Sealed {}

/// Specifies the memory strategy of the dynamic type.
///
/// For `Gd<Object>`, it is determined at runtime whether the instance is manually managed or ref-counted.
pub trait DynMemory: Sealed {
    /// Initialize reference counter
    #[doc(hidden)]
    fn maybe_init_ref<T: GodotClass>(obj: &RawGd<T>);

    /// If ref-counted, then increment count
    #[doc(hidden)]
    fn maybe_inc_ref<T: GodotClass>(obj: &RawGd<T>);

    /// If ref-counted, then decrement count. Returns `true` if the count hit 0 and the object can be
    /// safely freed.
    ///
    /// This behavior can be overriden by a script, making it possible for the function to return `false`
    /// even when the reference count hits 0. This is meant to be used to have a separate reference count
    /// from Godot's internal reference count, or otherwise stop the object from being freed when the
    /// reference count hits 0.
    ///
    /// # Safety
    ///
    /// If this method is used on a [`Gd`] that inherits from [`RefCounted`](crate::engine::RefCounted)
    /// then the reference count must either be incremented before it hits 0, or some [`Gd`] referencing
    /// this object must be forgotten.
    #[doc(hidden)]
    unsafe fn maybe_dec_ref<T: GodotClass>(obj: &RawGd<T>) -> bool;

    /// Check if ref-counted, return `None` if information is not available (dynamic and obj dead)
    #[doc(hidden)]
    fn is_ref_counted<T: GodotClass>(obj: &RawGd<T>) -> Option<bool>;

    /// Returns `true` if argument and return pointers are passed as `Ref<T>` pointers given this
    /// [`PtrcallType`].
    ///
    /// See [`PtrcallType::Virtual`] for information about `Ref<T>` objects.
    #[doc(hidden)]
    fn pass_as_ref(_call_type: sys::PtrcallType) -> bool {
        false
    }
}

/// Memory managed through Godot reference counter (always present).
/// This is used for `RefCounted` classes and derived.
pub struct MemRefCounted {}
impl Sealed for MemRefCounted {}
impl Memory for MemRefCounted {}
impl DynMemory for MemRefCounted {
    fn maybe_init_ref<T: GodotClass>(obj: &RawGd<T>) {
        out!("  Stat::init  <{}>", std::any::type_name::<T>());
        if obj.is_null() {
            return;
        }
        obj.as_ref_counted(|refc| {
            let success = refc.init_ref();
            assert!(success, "init_ref() failed");
        });
    }

    fn maybe_inc_ref<T: GodotClass>(obj: &RawGd<T>) {
        out!("  Stat::inc   <{}>", std::any::type_name::<T>());
        if obj.is_null() {
            return;
        }
        obj.as_ref_counted(|refc| {
            let success = refc.reference();
            assert!(success, "reference() failed");
        });
    }

    unsafe fn maybe_dec_ref<T: GodotClass>(obj: &RawGd<T>) -> bool {
        out!("  Stat::dec   <{}>", std::any::type_name::<T>());
        if obj.is_null() {
            return false;
        }
        obj.as_ref_counted(|refc| {
            let is_last = refc.unreference();
            out!("  +-- was last={is_last}");
            is_last
        })
    }

    fn is_ref_counted<T: GodotClass>(_obj: &RawGd<T>) -> Option<bool> {
        Some(true)
    }

    fn pass_as_ref(call_type: sys::PtrcallType) -> bool {
        matches!(call_type, sys::PtrcallType::Virtual)
    }
}

/// Memory managed through Godot reference counter, if present; otherwise manual.
/// This is used only for `Object` classes.
pub struct MemDynamic {}
impl Sealed for MemDynamic {}
impl DynMemory for MemDynamic {
    fn maybe_init_ref<T: GodotClass>(obj: &RawGd<T>) {
        out!("  Dyn::init  <{}>", std::any::type_name::<T>());
        if obj
            .instance_id_unchecked()
            .map(|id| id.is_ref_counted())
            .unwrap_or(false)
        {
            // Will call `RefCounted::init_ref()` which checks for liveness.
            MemRefCounted::maybe_init_ref(obj)
        }
    }

    fn maybe_inc_ref<T: GodotClass>(obj: &RawGd<T>) {
        out!("  Dyn::inc   <{}>", std::any::type_name::<T>());
        if obj
            .instance_id_unchecked()
            .map(|id| id.is_ref_counted())
            .unwrap_or(false)
        {
            // Will call `RefCounted::reference()` which checks for liveness.
            MemRefCounted::maybe_inc_ref(obj)
        }
    }

    unsafe fn maybe_dec_ref<T: GodotClass>(obj: &RawGd<T>) -> bool {
        out!("  Dyn::dec   <{}>", std::any::type_name::<T>());
        if obj
            .instance_id_unchecked()
            .map(|id| id.is_ref_counted())
            .unwrap_or(false)
        {
            // Will call `RefCounted::unreference()` which checks for liveness.
            MemRefCounted::maybe_dec_ref(obj)
        } else {
            false
        }
    }

    fn is_ref_counted<T: GodotClass>(obj: &RawGd<T>) -> Option<bool> {
        // Return `None` if obj is dead
        obj.instance_id_unchecked().map(|id| id.is_ref_counted())
    }
}

/// No memory management, user responsible for not leaking.
/// This is used for all `Object` derivates, which are not `RefCounted`. `Object` itself is also excluded.
pub struct MemManual {}
impl Sealed for MemManual {}
impl Memory for MemManual {}
impl DynMemory for MemManual {
    fn maybe_init_ref<T: GodotClass>(_obj: &RawGd<T>) {}
    fn maybe_inc_ref<T: GodotClass>(_obj: &RawGd<T>) {}
    unsafe fn maybe_dec_ref<T: GodotClass>(_obj: &RawGd<T>) -> bool {
        false
    }
    fn is_ref_counted<T: GodotClass>(_obj: &RawGd<T>) -> Option<bool> {
        Some(false)
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Declarer bounds

/// Trait that specifies who declares a given `GodotClass`.
pub trait Declarer: Sealed {
    type DerefTarget<T: GodotClass>;

    #[doc(hidden)]
    fn scoped_mut<T, F, R>(obj: &mut RawGd<T>, closure: F) -> R
    where
        T: GodotClass + Bounds<Declarer = Self>,
        F: FnOnce(&mut T) -> R;

    /// Check if the object is a user object *and* currently locked by a `bind()` or `bind_mut()` guard.
    ///
    /// # Safety
    /// Object must be alive.
    #[doc(hidden)]
    unsafe fn is_currently_bound<T>(obj: &RawGd<T>) -> bool
    where
        T: GodotClass + Bounds<Declarer = Self>;

    #[doc(hidden)]
    fn create_gd<T>() -> Gd<T>
    where
        T: GodotDefault + Bounds<Declarer = Self>;
}

/// Expresses that a class is declared by the Godot engine.
pub enum DeclEngine {}
impl Sealed for DeclEngine {}
impl Declarer for DeclEngine {
    type DerefTarget<T: GodotClass> = T;

    fn scoped_mut<T, F, R>(obj: &mut RawGd<T>, closure: F) -> R
    where
        T: GodotClass + Bounds<Declarer = DeclEngine>,
        F: FnOnce(&mut T) -> R,
    {
        closure(
            obj.as_target_mut()
                .expect("scoped mut should not be called on a null object"),
        )
    }

    unsafe fn is_currently_bound<T>(_obj: &RawGd<T>) -> bool
    where
        T: GodotClass + Bounds<Declarer = Self>,
    {
        false
    }

    fn create_gd<T>() -> Gd<T>
    where
        T: GodotDefault + Bounds<Declarer = Self>,
    {
        unsafe {
            let object_ptr =
                sys::interface_fn!(classdb_construct_object)(T::class_name().string_sys());
            Gd::from_obj_sys(object_ptr)
        }
    }
}

/// Expresses that a class is declared by the user.
pub enum DeclUser {}
impl Sealed for DeclUser {}
impl Declarer for DeclUser {
    type DerefTarget<T: GodotClass> = T::Base;

    fn scoped_mut<T, F, R>(obj: &mut RawGd<T>, closure: F) -> R
    where
        T: GodotClass + Bounds<Declarer = Self>,
        F: FnOnce(&mut T) -> R,
    {
        let mut guard = obj.bind_mut();
        closure(&mut *guard)
    }

    unsafe fn is_currently_bound<T>(obj: &RawGd<T>) -> bool
    where
        T: GodotClass + Bounds<Declarer = Self>,
    {
        obj.storage().unwrap_unchecked().is_bound()
    }

    fn create_gd<T>() -> Gd<T>
    where
        T: GodotDefault + Bounds<Declarer = Self>,
    {
        unsafe {
            let object_ptr = callbacks::create::<T>(std::ptr::null_mut());
            Gd::from_obj_sys(object_ptr)
        }
    }
}
