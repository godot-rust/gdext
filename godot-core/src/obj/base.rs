/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::obj::Gd;
use crate::obj::GodotClass;
use crate::{engine, sys};
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::mem::ManuallyDrop;

/// Restricted version of `Gd`, to hold the base instance inside a user's `GodotClass`.
///
/// Behaves similarly to [`Gd`][crate::obj::Gd], but is more constrained. Cannot be constructed by the user.
pub struct Base<T: GodotClass> {
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
    /// # Safety
    /// The returned Base is a weak pointer, so holding it will not keep the object alive. It must not be accessed after the object is destroyed.
    pub(crate) unsafe fn from_base(base: &Base<T>) -> Base<T> {
        Base::from_obj(Gd::from_obj_sys_weak(base.as_gd().obj_sys()))
    }

    // Note: not &mut self, to only borrow one field and not the entire struct
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

    /// Returns a [`Gd`] referencing the same object as this reference.
    ///
    /// Using this method to call methods on the base field of a Rust object is discouraged, instead use the
    /// methods from [`WithBaseField`](super::WithBaseField) when possible.
    #[doc(hidden)]
    pub fn as_gd(&self) -> &Gd<T> {
        &self.obj
    }

    // Currently only used in outbound virtual calls (for scripts).
    #[doc(hidden)]
    pub fn obj_sys(&self) -> sys::GDExtensionObjectPtr {
        self.obj.obj_sys()
    }
}

impl<T: GodotClass> Debug for Base<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        engine::debug_string(&self.obj, f, "Base")
    }
}

impl<T: GodotClass> Display for Base<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        engine::display_string(&self.obj, f)
    }
}
