/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::mem::ManuallyDrop;

use crate::obj::base_init::{InitState, InitTracker};
use crate::obj::{BorrowedGd, Gd, GodotClass};
use crate::{classes, sys};

macro_rules! base_from_obj {
    ($obj:expr_2021, $state:expr_2021) => {
        Base::from_obj($obj, $state)
    };
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Restricted version of `Gd`, to hold the base instance inside a user's `GodotClass`.
///
/// Behaves similarly to [`Gd`][crate::obj::Gd], but is more constrained. Cannot be constructed by the user.
pub struct Base<T: GodotClass> {
    // Like `Gd`, it's theoretically possible that Base is destroyed while there are still other Gd pointers to the underlying object. This is
    // safe, may however lead to unintended behavior. The base_test.rs file checks some of these scenarios.

    // Internal smart pointer is never dropped. It thus acts like a weak pointer and is needed to break reference cycles between Gd<T>
    // and the user instance owned by InstanceStorage.
    //
    // There is no data apart from the opaque bytes, so no memory or resources to deallocate.
    // When triggered by Godot/GDScript, the destruction order is as follows:
    // 1.    Most-derived Godot class (C++)
    //      ...
    // 2.  RefCounted (C++)
    // 3. Object (C++) -- this triggers InstanceStorage destruction
    // 4.   Base<T>
    // 5.  User struct (GodotClass implementation)
    // 6. InstanceStorage
    //
    // When triggered by Rust (Gd::drop on last strong ref), it's as follows:
    // 1.   Gd<T>  -- triggers InstanceStorage destruction
    // 2.
    pub(super) obj: ManuallyDrop<Gd<T>>,

    pub(super) init_state: InitTracker,
}

impl<T: GodotClass> Base<T> {
    /// "Copy constructor": allows to share a `Base<T>` weak pointer.
    ///
    /// The return value is a weak pointer, so it will not keep the instance alive.
    ///
    /// # Safety
    /// `base` must be alive at the time of invocation, i.e. user `init()` (which could technically destroy it) must not have run yet.
    /// If `base` is destroyed while the returned `Base<T>` is in use, that constitutes a logic error, not a safety issue.
    pub(crate) unsafe fn from_base(base: &Base<T>) -> Base<T> {
        sys::balanced_assert!(
            Base::is_valid(base),
            "Cannot construct Base; was object freed during initialization?"
        );

        // SAFETY: See method docs.
        let obj = unsafe { Gd::from_obj_sys_weak(base.obj.obj_sys()) };

        Self {
            obj: ManuallyDrop::new(obj),
            init_state: base.init_state.clone(),
        }
    }

    /// Checks if this base points to a live object.
    pub(crate) fn is_valid(base: &Base<T>) -> bool {
        base.obj.is_instance_valid()
    }

    /// Workaround for `base` being unitialized during object initialization and `NOTIFICATION_POSTINITIALIZE`
    /// for Godot versions before 4.7.
    ///
    /// # Behaviour after Godot 4.7
    ///
    /// Since Godot 4.7 initialization layer receives fully-constructed object to work with – therefore in Godot 4.7 and later
    /// this method simply returns a clone of a given instance.
    ///
    /// Use this method if you want to support Godot versions older than 4.7.
    ///
    /// # Behaviour before Godot 4.7
    ///
    /// Returns a [`Gd`] referencing the base object, for exclusive use during object initialization and `NOTIFICATION_POSTINITIALIZE`.
    ///
    /// Can be used during an initialization function [`I*::init()`][crate::classes::IObject::init] or [`Gd::from_init_fn()`], or [`POSTINITIALIZE`][crate::classes::notify::ObjectNotification::POSTINITIALIZE].
    ///
    /// The base pointer is only pointing to a base object; you cannot yet downcast it to the object being constructed.
    /// The instance ID is the same as the one the in-construction object will have.
    ///
    /// ## Lifecycle for ref-counted classes
    ///
    /// If `T: Inherits<RefCounted>`, then the ref-counted object is not yet fully-initialized at the time of the `init` function and [`POSTINITIALIZE`][crate::classes::notify::ObjectNotification::POSTINITIALIZE] running.
    /// Accessing the base object without further measures would be dangerous. Here, godot-rust employs a workaround: the `Base` object (which
    /// holds a weak pointer to the actual instance) is temporarily upgraded to a strong pointer, preventing use-after-free.
    ///
    /// This additional reference is automatically dropped at an implementation-defined point in time (which may change, and technically delay
    /// destruction of your object as soon as you use `Base::to_init_gd()`). Right now, this refcount-decrement is deferred to the next frame.
    ///
    /// Ref-counted bases can only use `to_init_gd()` on the main thread.
    ///
    /// ## Panics (Debug)
    /// In Godot before 4.7, if called outside an initialization function, or for ref-counted objects on a non-main thread.
    pub fn to_init_gd(&self) -> Gd<T> {
        self.to_init_gd_inner()
    }

    /// Create base from existing object (used in script instances).
    ///
    /// The return value is a weak pointer, so it will not keep the instance alive.
    ///
    /// # Safety
    /// `gd` must be alive at the time of invocation. If it is destroyed while the returned `Base<T>` is in use, that constitutes a logic
    /// error, not a safety issue.
    pub(crate) unsafe fn from_script_gd(gd: &Gd<T>) -> Self {
        sys::balanced_assert!(gd.is_instance_valid());

        // SAFETY: pointer is valid and remains alive while in use.
        let obj = unsafe { Gd::from_obj_sys_weak(gd.obj_sys()) };

        base_from_obj!(obj, InitState::Script)
    }

    /// Create new base from raw Godot object.
    ///
    /// The return value is a weak pointer, so it will not keep the instance alive.
    ///
    /// # Safety
    /// `base_ptr` must point to a valid, live object at the time of invocation. If it is destroyed while the returned `Base<T>` is in use,
    /// that constitutes a logic error, not a safety issue.
    pub(crate) unsafe fn from_sys(base_ptr: sys::GDExtensionObjectPtr) -> Self {
        sys::balanced_assert!(!base_ptr.is_null(), "instance base is null pointer");

        // Initialize only as weak pointer (don't increment reference count).
        // SAFETY: pointer is valid and remains alive while in use.
        let obj = unsafe { Gd::from_obj_sys_weak(base_ptr) };

        // This obj does not contribute to the strong count, otherwise we create a reference cycle:
        // 1. RefCounted (dropped in GDScript)
        // 2. holds user T (via extension instance and storage)
        // 3. holds Base<T> RefCounted (last ref, dropped in T destructor, but T is never destroyed because this ref keeps storage alive)
        // Note that if late-init never happened on self, we have the same behavior (still a raw pointer instead of weak Gd)
        base_from_obj!(obj, InitState::ObjectConstructing)
    }

    fn from_obj(obj: Gd<T>, init_state: InitState) -> Self {
        Self {
            obj: ManuallyDrop::new(obj),
            init_state: InitTracker::new(init_state),
        }
    }

    /// Returns a [`Gd`] referencing the base object, for use in script contexts only.
    #[doc(hidden)]
    pub fn __script_gd(&self) -> Gd<T> {
        // Used internally by `SiMut::base()` and `SiMut::base_mut()` for script re-entrancy.
        // Could maybe add debug validation to ensure script context in the future.
        (*self.obj).clone()
    }

    // Currently only used in outbound virtual calls (for scripts); search for: base_field(self).obj_sys().
    #[doc(hidden)]
    pub fn obj_sys(&self) -> sys::GDExtensionObjectPtr {
        self.obj.obj_sys()
    }

    // Internal use only, do not make public.
    #[cfg(feature = "debug-log")] #[cfg_attr(published_docs, doc(cfg(feature = "debug-log")))]
    pub(crate) fn debug_instance_id(&self) -> crate::obj::InstanceId {
        self.obj.instance_id()
    }

    /// Returns a borrowed reference to the base object, for use in script contexts only.
    pub(crate) fn to_script_borrowed(&self) -> BorrowedGd<'_, T> {
        self.init_state.assert_script();

        BorrowedGd::from_gd(&self.obj)
    }

    /// Returns a [`Gd`] referencing the base object, assuming the derived object is fully constructed.
    #[doc(hidden)]
    pub fn __constructed_gd(&self) -> Gd<T> {
        self.init_state.assert_constructed();
        (*self.obj).clone()
    }

    /// Returns a [`BorrowedGd`] referencing the base object, assuming the derived object is fully constructed.
    ///
    /// Unlike [`Self::__constructed_gd()`], this does not increment the reference count for ref-counted `T`s.
    pub(crate) fn constructed_borrowed(&self) -> BorrowedGd<'_, T> {
        self.init_state.assert_constructed();

        BorrowedGd::from_gd(&self.obj)
    }

    /// Returns the raw object pointer of the base, assuming the derived object is fully constructed.
    ///
    /// Used by `base_mut()`, which cannot use [`Self::constructed_borrowed()`]: holding its `&self` borrow would conflict with the
    /// mutable instance borrow taken right after. The raw pointer carries no borrow; `base_mut()` constructs the `BorrowedGd` later,
    /// with its lifetime tied to the instance guard.
    pub(crate) fn constructed_obj_sys(&self) -> sys::GDExtensionObjectPtr {
        self.init_state.assert_constructed();

        self.obj.obj_sys()
    }
}

impl<T: GodotClass> Debug for Base<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        classes::debug_string(&self.obj, f, "Base")
    }
}

impl<T: GodotClass> Display for Base<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        classes::display_string(&self.obj, f)
    }
}
