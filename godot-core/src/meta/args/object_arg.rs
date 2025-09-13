/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::ptr;

use godot_ffi::{ExtVariantType, GodotFfi, GodotNullableFfi, PtrcallType};

use crate::builtin::Variant;
use crate::meta::error::ConvertError;
use crate::meta::traits::GodotFfiVariant;
use crate::obj::{Gd, GodotClass};
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

    /// Creates `ObjectArg` from an `Option<Gd>`.
    ///
    /// # Safety
    /// The referenced `Gd`, if not `None`, must remain valid for the lifetime of this `ObjectArg`.
    pub unsafe fn from_option_gd<T: GodotClass>(obj: Option<&Gd<T>>) -> Self {
        match obj {
            Some(gd) => Self::from_gd(gd),
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

impl GodotFfiVariant for ObjectArg {
    fn ffi_to_variant(&self) -> Variant {
        obj::object_ffi_to_variant(self)
    }

    fn ffi_from_variant(_variant: &Variant) -> Result<Self, ConvertError> {
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
