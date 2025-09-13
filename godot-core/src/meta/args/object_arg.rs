/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::ptr;

use godot_ffi::{ExtVariantType, GodotFfi, GodotNullableFfi, PtrcallType};

use crate::obj::{Gd, GodotClass, Inherits, RawGd};
use crate::{obj, sys};

/// View for object arguments passed to the Godot engine. Never owning; must be null or backed by `Gd<T>`.
///
/// This type stores only an untyped object pointer, allowing efficient borrowing for object arguments
/// without cloning. It supports nullable object passing for optional object arguments.
#[derive(Debug, PartialEq)]
#[doc(hidden)]
pub struct ObjectArg {
    // Never dropped since it's just a view; see constructor.
    object_ptr: sys::GDExtensionObjectPtr,
}

impl ObjectArg {
    /// Creates `ObjectArg` from a `Gd`.
    ///
    /// # Safety
    /// The referenced `Gd` must remain valid for the lifetime of this `ObjectArg`.
    pub unsafe fn from_gd<T: GodotClass>(obj: &Gd<T>) -> Self {
        Self {
            object_ptr: obj.obj_sys(),
        }
    }

    /// Creates `ObjectArg` from a `RawGd`.
    ///
    /// # Safety
    /// The referenced `RawGd` must remain valid for the lifetime of this `ObjectArg`.
    pub unsafe fn from_raw_gd<T: GodotClass>(obj: &RawGd<T>) -> Self {
        // Runtime check is necessary, to ensure that object is still alive and has correct runtime type.
        if !obj.is_null() {
            obj.check_rtti("from_raw_gd");
        }

        Self {
            object_ptr: obj.obj_sys(),
        }
    }

    /// Creates `ObjectArg` from `Option<&Gd<U>>`, handling upcast to target type `T`.
    pub fn from_option_gd_ref<T, U>(opt: Option<&Gd<U>>) -> Self
    where
        T: GodotClass,
        U: GodotClass + Inherits<T>,
    {
        match opt {
            Some(gd) => unsafe { Self::from_gd(gd) },
            None => Self::null(),
        }
    }

    /// Creates a null ObjectArg
    pub fn null() -> Self {
        Self {
            object_ptr: ptr::null_mut(),
        }
    }

    /// Returns true if this ObjectArg represents null
    pub fn is_null(&self) -> bool {
        self.object_ptr.is_null()
    }

    /// Returns the raw object pointer
    pub fn raw_ptr(&self) -> sys::GDExtensionObjectPtr {
        self.object_ptr
    }
}

// #[derive(Clone)] doesn't seem to get bounds right.
impl Clone for ObjectArg {
    fn clone(&self) -> Self {
        Self {
            object_ptr: self.object_ptr,
        }
    }
}

// SAFETY: see impl GodotFfi for RawGd.
unsafe impl GodotFfi for ObjectArg {
    // If anything changes here, keep in sync with RawGd impl.

    const VARIANT_TYPE: ExtVariantType = ExtVariantType::Concrete(sys::VariantType::OBJECT);

    unsafe fn new_from_sys(_ptr: sys::GDExtensionConstTypePtr) -> Self {
        unreachable!("ObjectArg should only be passed *to* Godot, not *from*.")
    }

    unsafe fn new_with_uninit(_init: impl FnOnce(sys::GDExtensionUninitializedTypePtr)) -> Self {
        unreachable!("ObjectArg should only be passed *to* Godot, not *from*.")
    }

    unsafe fn new_with_init(_init: impl FnOnce(sys::GDExtensionTypePtr)) -> Self {
        unreachable!("ObjectArg should only be passed *to* Godot, not *from*.")
    }

    fn sys(&self) -> sys::GDExtensionConstTypePtr {
        self.object_ptr.cast()
    }

    fn sys_mut(&mut self) -> sys::GDExtensionTypePtr {
        self.object_ptr.cast()
    }

    // For more context around `ref_get_object` and `ref_set_object`, see:
    // https://github.com/godotengine/godot-cpp/issues/954

    fn as_arg_ptr(&self) -> sys::GDExtensionConstTypePtr {
        obj::object_as_arg_ptr(&self.object_ptr)
    }

    unsafe fn from_arg_ptr(_ptr: sys::GDExtensionTypePtr, _call_type: PtrcallType) -> Self {
        unreachable!("ObjectArg should only be passed *to* Godot, not *from*.")
    }

    unsafe fn move_return_ptr(self, _ptr: sys::GDExtensionTypePtr, _call_type: PtrcallType) {
        unreachable!("ObjectArg should only be passed *to* Godot, not *from*.")
    }
}

impl GodotNullableFfi for ObjectArg {
    fn null() -> Self {
        Self::null()
    }

    fn is_null(&self) -> bool {
        Self::is_null(self)
    }
}
