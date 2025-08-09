/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::mem::ManuallyDrop;

use crate::obj::{Gd, GodotClass};
use crate::{classes, sys};

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
    obj: ManuallyDrop<Gd<T>>,
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
        debug_assert!(base.obj.is_instance_valid());
        Base::from_obj(Gd::from_obj_sys_weak(base.obj.obj_sys()))
    }

    /// Create base from existing object (used in script instances).
    ///
    /// The return value is a weak pointer, so it will not keep the instance alive.
    ///
    /// # Safety
    /// `gd` must be alive at the time of invocation. If it is destroyed while the returned `Base<T>` is in use, that constitutes a logic
    /// error, not a safety issue.
    pub(crate) unsafe fn from_gd(gd: &Gd<T>) -> Self {
        debug_assert!(gd.is_instance_valid());
        Base::from_obj(Gd::from_obj_sys_weak(gd.obj_sys()))
    }

    /// Create new base from raw Godot object.
    ///
    /// The return value is a weak pointer, so it will not keep the instance alive.
    ///
    /// # Safety
    /// `base_ptr` must point to a valid, live object at the time of invocation. If it is destroyed while the returned `Base<T>` is in use,
    /// that constitutes a logic error, not a safety issue.
    pub(crate) unsafe fn from_sys(base_ptr: sys::GDExtensionObjectPtr) -> Self {
        assert!(!base_ptr.is_null(), "instance base is null pointer");

        // Initialize only as weak pointer (don't increment reference count)
        let obj = Gd::from_obj_sys_weak(base_ptr);

        // This obj does not contribute to the strong count, otherwise we create a reference cycle:
        // 1. RefCounted (dropped in GDScript)
        // 2. holds user T (via extension instance and storage)
        // 3. holds Base<T> RefCounted (last ref, dropped in T destructor, but T is never destroyed because this ref keeps storage alive)
        // Note that if late-init never happened on self, we have the same behavior (still a raw pointer instead of weak Gd)
        Base::from_obj(obj)
    }

    fn from_obj(obj: Gd<T>) -> Self {
        Self {
            obj: ManuallyDrop::new(obj),
        }
    }

    /// Returns a [`Gd`] referencing the same object as this reference.
    ///
    /// Using this method to call methods on the base field of a Rust object is discouraged, instead use the
    /// methods from [`WithBaseField`](super::WithBaseField) when possible.
    #[doc(hidden)]
    pub fn to_gd(&self) -> Gd<T> {
        (*self.obj).clone()
    }

    // Currently only used in outbound virtual calls (for scripts); search for: base_field(self).obj_sys().
    #[doc(hidden)]
    pub fn obj_sys(&self) -> sys::GDExtensionObjectPtr {
        self.obj.obj_sys()
    }

    // Internal use only, do not make public.
    #[cfg(feature = "debug-log")]
    pub(crate) fn debug_instance_id(&self) -> crate::obj::InstanceId {
        self.obj.instance_id()
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
