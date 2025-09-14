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

/// Passive (non-owning) reference to a Godot object.
///
/// `PassiveGd<'gd, T>` provides a safe abstraction for weak references to Godot objects. Unlike `Gd<T>`, it does not increment/decrement
/// the reference count for `RefCounted` objects, and its `Drop` impl only cleans up metadata, not the Godot object.
///
/// The lifetime `'gd` can be used to tie it to a _strong_ `Gd<T>` reference, however it can also be `'static` if more flexibility is needed.
///
/// This type is primarily used internally for base object access in guards and traits, providing a clean alternative to manual
/// [`Gd::clone_weak()`] and [`Gd::drop_weak()`] patterns.
pub(crate) struct PassiveGd<'gd, T: GodotClass> {
    weak_gd: ManuallyDrop<Gd<T>>,

    // Covariant lifetime: PassiveGd<'a, T> can be used wherever PassiveGd<'b, T> is needed, if 'a: 'b.
    _phantom: PhantomData<&'gd ()>,
}

impl<'gd, T: GodotClass> PassiveGd<'gd, T> {
    pub fn from_strong_ref(gd: &Gd<T>) -> Self {
        // SAFETY:
        // - `clone_weak()` creates a pointer conforming to `from_weak_gd()` requirements.
        // - PassiveGd will destroy the pointer with `drop_weak()`.
        unsafe {
            let weak_gd = gd.clone_weak();
            Self::from_weak_owned(weak_gd)
        }
    }

    /// Creates a passive reference directly from a weak `Gd<T>`.
    ///
    /// Will invoke `Gd::drop_weak()` when dropped. Since the parameter has no lifetime, you need to provide the lifetime `'gd` explicitly.
    ///
    /// # Safety
    /// - `weak_gd` must be a weakly created `Gd`, e.g. from [`Gd::clone_weak()`] or [`Gd::from_obj_sys_weak()`].
    /// - The caller must ensure that the `weak_gd` remains valid for the lifetime `'gd`.
    pub unsafe fn from_weak_owned(weak_gd: Gd<T>) -> Self {
        Self {
            weak_gd: ManuallyDrop::new(weak_gd),
            _phantom: PhantomData,
        }
    }
}

impl<T: GodotClass> Drop for PassiveGd<'_, T> {
    fn drop(&mut self) {
        // SAFETY: Only extracted once, in Drop.
        let weak = unsafe { ManuallyDrop::take(&mut self.weak_gd) };

        weak.drop_weak();
    }
}

impl<T: GodotClass> Deref for PassiveGd<'_, T> {
    type Target = Gd<T>;

    fn deref(&self) -> &Self::Target {
        &self.weak_gd
    }
}

impl<T: GodotClass> DerefMut for PassiveGd<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.weak_gd
    }
}

// Note: We intentionally do NOT implement Clone for PassiveGd, as cloning weak references requires careful lifetime management that
// should be explicit.
