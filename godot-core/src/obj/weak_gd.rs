/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::fmt::{Debug, Formatter, Result as FmtResult};
use std::mem::ManuallyDrop;
use std::ops::{Deref, DerefMut};

use godot_ffi as sys;

use crate::classes;
use crate::obj::{bounds, Base, Bounds, Gd, GdDerefTarget, GodotClass, RawGd};

/// Weak pointer to objects owned by the Godot engine.
/// `WeakGd<T>` doesn't guarantee the validation of the object and can hold a dead object.
/// For `RefCounted`, it doesn't affect the reference count, which means it doesn't decrease reference count when dropped and doesn't prevent the `RefCounted` from being released.
/// Can be used during initialization [`I*::init()`][crate::classes::IObject::init] or [`Gd::from_init_fn()`], and deconstruction [Drop].
///
/// # Panics
/// If the weak pointer is invalid when dereferencing to call a Godot method.
pub struct WeakGd<T: GodotClass> {
    raw: Option<ManuallyDrop<RawGd<T>>>,
}

impl<T: GodotClass> WeakGd<T> {
    #[doc(hidden)]
    pub unsafe fn from_obj_sys_weak(val: sys::GDExtensionObjectPtr) -> Self {
        Self {
            raw: if !val.is_null() {
                Some(ManuallyDrop::new(RawGd::from_obj_sys_weak(val)))
            } else {
                None
            },
        }
    }

    pub(crate) fn from_raw_gd(val: &RawGd<T>) -> Self {
        unsafe {
            Self::from_obj_sys_weak(if val.is_instance_valid() {
                val.obj_sys()
            } else {
                std::ptr::null_mut()
            })
        }
    }

    /// Create a weak pointer from a [Gd].
    pub fn from_gd(val: &Gd<T>) -> Self {
        Self::from_raw_gd(&val.raw)
    }

    /// Create a weak pointer from a [Base].
    pub fn from_base(val: &Base<T>) -> Self {
        Self {
            raw: if val.is_instance_valid() {
                unsafe { Some(ManuallyDrop::new(RawGd::from_obj_sys_weak(val.obj_sys()))) }
            } else {
                None
            },
        }
    }

    /// Checks if this weak pointer points to a live object.
    pub fn is_instance_valid(&self) -> bool {
        self.raw
            .as_ref()
            .map(|v| v.is_instance_valid())
            .unwrap_or(false)
    }
}

impl<T: GodotClass> Deref for WeakGd<T>
where
    GdDerefTarget<T>: Bounds<Declarer = bounds::DeclEngine>,
{
    type Target = GdDerefTarget<T>;

    fn deref(&self) -> &Self::Target {
        self.raw
            .as_ref()
            .expect("WeakGd points to an invalid instance")
            .as_target()
    }
}

impl<T: GodotClass> DerefMut for WeakGd<T>
where
    GdDerefTarget<T>: Bounds<Declarer = bounds::DeclEngine>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.raw
            .as_mut()
            .expect("WeakGd points to an invalid instance")
            .as_target_mut()
    }
}

impl<T: GodotClass> Clone for WeakGd<T> {
    fn clone(&self) -> Self {
        if let Some(raw) = self.raw.as_ref() {
            Self::from_raw_gd(raw)
        } else {
            Self { raw: None }
        }
    }
}

impl<T: GodotClass> Debug for WeakGd<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        if self.is_instance_valid() {
            classes::debug_string_nullable(self.raw.as_ref().unwrap(), f, "WeakGd")
        } else {
            write!(f, "WeakGd {{ null }}")
        }
    }
}
