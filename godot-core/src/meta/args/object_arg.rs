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
pub struct ObjectArg<'gd> {
    // Never dropped since it's just a view; see constructor.
    object_ptr: sys::GDExtensionObjectPtr,
    _lifetime: std::marker::PhantomData<&'gd ()>,
}

impl<'gd> ObjectArg<'gd> {
    /// Creates a temporary `ObjectArg` from a `Gd` reference.
    pub fn from_gd<T: GodotClass>(obj: &'gd Gd<T>) -> Self {
        Self {
            object_ptr: obj.obj_sys(),
            _lifetime: std::marker::PhantomData,
        }
    }

    /// Creates a temporary `ObjectArg` from an `Option<&Gd>` reference.
    pub fn from_option_gd<T: GodotClass>(obj: Option<&'gd Gd<T>>) -> Self {
        match obj {
            Some(gd) => Self::from_gd(gd),
            None => Self::null(),
        }
    }

    /// Creates a null ObjectArg
    pub fn null() -> Self {
        Self {
            object_ptr: ptr::null_mut(),
            _lifetime: std::marker::PhantomData,
        }
    }

    /// Returns true if this ObjectArg represents null
    pub fn is_null(&self) -> bool {
        self.object_ptr.is_null()
    }

    /// Returns the raw object pointer.
    pub fn obj_sys(&self) -> sys::GDExtensionObjectPtr {
        self.object_ptr
    }
}

// #[derive(Clone)] doesn't seem to get bounds right.
impl<'gd> Clone for ObjectArg<'gd> {
    fn clone(&self) -> Self {
        Self {
            object_ptr: self.object_ptr,
            _lifetime: std::marker::PhantomData,
        }
    }
}

// SAFETY: see impl GodotFfi for RawGd.
unsafe impl<'gd> GodotFfi for ObjectArg<'gd> {
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

impl<'gd> GodotFfiVariant for ObjectArg<'gd> {
    fn ffi_to_variant(&self) -> Variant {
        obj::object_ffi_to_variant(self)
    }

    fn ffi_from_variant(_variant: &Variant) -> Result<Self, ConvertError> {
        unreachable!("ObjectArg should only be passed *to* Godot, not *from*.")
    }
}

impl<'gd> GodotNullableFfi for ObjectArg<'gd> {
    fn null() -> Self {
        Self::null()
    }

    fn is_null(&self) -> bool {
        Self::is_null(self)
    }
}
