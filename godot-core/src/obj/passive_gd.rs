/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::mem::ManuallyDrop;
use std::ops::{Deref, DerefMut};

use crate::obj::{Gd, GodotClass};
use crate::sys;

/// Passive (non-owning) reference to a Godot object.
///
/// `PassiveGd<T>` provides an unsafe abstraction for weak references to Godot objects. Unlike `Gd<T>`, it does not increment/decrement
/// the reference count for `RefCounted` objects, and its `Drop` impl only cleans up metadata, not the Godot object.
///
/// This type is for internal use only, to access base objects in guards and traits, and to wrap manual [`Gd::clone_weak()`] and
/// [`Gd::drop_weak()`] patterns.
///
/// # Why no lifetime?
/// Previous versions used `PassiveGd<'gd, T>` with an explicit lifetime parameter. This caused subtle borrow-checking issues due to Rust's
/// [drop check](https://doc.rust-lang.org/nomicon/dropck.html) rules. When a type has drop obligations (implements `Drop` or contains fields
/// that do), the borrow checker conservatively assumes the destructor might access borrowed data reachable through that value, forcing all
/// such borrows to strictly outlive the value. This created false conflicts when creating both shared and mutable base references from the
/// same object, even though our `Drop` implementation never accesses the lifetime-bound data.
///
/// By removing the lifetime parameter and making construction `unsafe`, we eliminate these false-positive borrow conflicts while maintaining
/// memory safety through explicit caller contracts.
///
/// In nightly Rust, `#[may_dangle]` on the lifetime parameter might be an alternative, to tell the compiler that our `Drop` implementation
/// won't access the borrowed data, but this attribute requires careful safety analysis to ensure it's correctly applied.
pub(crate) struct PassiveGd<T: GodotClass> {
    weak_gd: ManuallyDrop<Gd<T>>,
}

impl<T: GodotClass> PassiveGd<T> {
    /// Creates a passive reference from a strong `Gd<T>` shared reference.
    ///
    /// # Safety
    /// The caller must ensure that the underlying object remains valid for the entire lifetime of this `PassiveGd`.
    pub unsafe fn from_strong_ref(gd: &Gd<T>) -> Self {
        // SAFETY: clone_weak() creates valid weak reference; caller ensures object validity.
        let weak_gd = gd.clone_weak();
        unsafe { Self::new(weak_gd) }
    }

    /// Creates a passive reference directly from a raw object pointer.
    ///
    /// This is a direct constructor that avoids the intermediate `Gd::from_obj_sys_weak()` step,
    /// providing better performance for the common pattern of creating PassiveGd from raw pointers.
    ///
    /// # Safety
    /// - `obj_ptr` must be a valid, live object pointer.
    /// - The caller must ensure that the underlying object remains valid for the entire lifetime of this `PassiveGd`.
    pub unsafe fn from_obj_sys(obj_ptr: sys::GDExtensionObjectPtr) -> Self {
        // SAFETY: from_obj_sys_weak() creates valid weak reference from obj_ptr; caller ensures object validity.
        unsafe {
            let weak_gd = Gd::from_obj_sys_weak(obj_ptr);
            Self::new(weak_gd)
        }
    }

    /// Creates a passive reference directly from a weak `Gd<T>`.
    ///
    /// Will invoke `Gd::drop_weak()` when dropped.
    ///
    /// # Safety
    /// - `weak_gd` must be a weakly created `Gd`, e.g. from [`Gd::clone_weak()`] or [`Gd::from_obj_sys_weak()`].
    /// - The caller must ensure that the underlying object remains valid for the entire lifetime of this `PassiveGd`.
    unsafe fn new(weak_gd: Gd<T>) -> Self {
        Self {
            weak_gd: ManuallyDrop::new(weak_gd),
        }
    }
}

impl<T: GodotClass> Drop for PassiveGd<T> {
    fn drop(&mut self) {
        // SAFETY: Only extracted once, in Drop.
        let weak = unsafe { ManuallyDrop::take(&mut self.weak_gd) };

        weak.drop_weak();
    }
}

impl<T: GodotClass> Deref for PassiveGd<T> {
    type Target = Gd<T>;

    fn deref(&self) -> &Self::Target {
        &self.weak_gd
    }
}

impl<T: GodotClass> DerefMut for PassiveGd<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.weak_gd
    }
}

// Note: We intentionally do NOT implement Clone for PassiveGd, as cloning weak references requires careful lifetime management that
// should be explicit.
