/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::marker::PhantomData;
use std::mem::ManuallyDrop;
use std::ops::{Deref, DerefMut};

use crate::obj::{Gd, GodotClass};
use crate::sys;

/// Borrowed (non-owning) reference to a Godot object.
///
/// Unlike `Gd<T>`, this type does not increment/decrement the reference count for `RefCounted` objects, and dropping it leaves the Godot
/// object untouched. The lifetime `'a` ties it to whatever guarantees the object's validity: a borrow of an existing `Gd` ([`Self::from_gd`]),
/// or a guard's borrow of the instance, unified at guard construction ([`Self::from_obj_sys`]).
///
/// This type is for internal use only, to access base objects in guards and traits. It wraps the manual [`Gd::clone_weak()`] +
/// [`Gd::drop_weak()`] pattern in a misuse-proof RAII API.
///
/// # Design: lifetimes vs. drop check
/// Rust's [drop check](https://doc.rust-lang.org/nomicon/dropck.html) requires that lifetimes _used by a type's destructor_ strictly outlive
/// the value, since the destructor might access the borrowed data. A `Drop` impl directly on `BorrowedGd<'a, T>` would therefore extend every
/// `'a` borrow until the end of scope, instead of ending it at the last use. Guards holding a `BorrowedGd` would then conflict with later
/// mutable borrows of the same object -- false positives, since our cleanup never accesses the borrowed data. (Nightly's `#[may_dangle]`
/// addresses this, but isn't available on stable.)
///
/// Instead, cleanup lives on the nested [`BorrowedStorage`], which has no lifetime parameter. Its destructor thus cannot "see" `'a`, so
/// borrows end at the last use, while cleanup still runs automatically. Future fields that need cleanup must go there, not into
/// `BorrowedGd` -- and `BorrowedGd` itself must never gain a `Drop` impl or a field whose destructor uses `'a`, as either would resurrect
/// the borrow conflicts.
pub(crate) struct BorrowedGd<'a, T: GodotClass> {
    storage: BorrowedStorage<T>,
    _borrow: PhantomData<&'a Gd<T>>,
}

/// Owning part of [`BorrowedGd`]: stores the weak `Gd` and cleans it up on drop. See there for why this is split out.
struct BorrowedStorage<T: GodotClass> {
    weak_gd: ManuallyDrop<Gd<T>>,
}

impl<T: GodotClass> Drop for BorrowedStorage<T> {
    fn drop(&mut self) {
        // SAFETY: only extracted once, in Drop.
        let weak = unsafe { ManuallyDrop::take(&mut self.weak_gd) };

        weak.drop_weak();
    }
}

impl<'a, T: GodotClass> BorrowedGd<'a, T> {
    /// Creates a borrowed reference from a `Gd<T>` shared reference.
    ///
    /// This is safe: the copy is only usable during `'a`, while `gd` is borrowed, and is thus exactly as valid as the original `Gd`.
    pub fn from_gd(gd: &'a Gd<T>) -> Self {
        // SAFETY: BorrowedStorage's Drop disposes of the weak copy via drop_weak(), leaving the reference count untouched.
        let weak_gd = unsafe { gd.clone_weak() };

        Self {
            storage: BorrowedStorage {
                weak_gd: ManuallyDrop::new(weak_gd),
            },
            _borrow: PhantomData,
        }
    }

    /// Creates a borrowed reference directly from a raw object pointer.
    ///
    /// Only needed when no `&'a Gd` is available to borrow from -- e.g. in `base_mut()`, where the instance is mutably borrowed and
    /// [`Self::from_gd`] would conflict with that borrow.
    ///
    /// # Safety
    /// - `obj_ptr` must be a valid, live object pointer.
    /// - The object must remain valid throughout `'a`. Since `'a` is not tied to any parameter, the caller must bind it to something
    ///   that guarantees this -- typically by passing the result to a guard constructor that unifies `'a` with an instance borrow.
    pub unsafe fn from_obj_sys(obj_ptr: sys::GDExtensionObjectPtr) -> Self {
        // SAFETY: obj_ptr is valid as per safety precondition of this fn.
        let weak_gd = unsafe { Gd::from_obj_sys_weak(obj_ptr) };

        Self {
            storage: BorrowedStorage {
                weak_gd: ManuallyDrop::new(weak_gd),
            },
            _borrow: PhantomData,
        }
    }
}

impl<T: GodotClass> Deref for BorrowedGd<'_, T> {
    type Target = Gd<T>;

    fn deref(&self) -> &Self::Target {
        &self.storage.weak_gd
    }
}

impl<T: GodotClass> DerefMut for BorrowedGd<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.storage.weak_gd
    }
}

// Note: We intentionally do NOT implement Clone for BorrowedGd, as cloning weak references requires careful lifetime management that
// should be explicit.
