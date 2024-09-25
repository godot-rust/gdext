/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
use crate::builtin::Variant;
use crate::meta::error::ConvertError;
use crate::meta::{FromGodot, GodotConvert, GodotFfiVariant, ToGodot};
use crate::sys;
use godot_ffi::{GodotFfi, GodotNullableFfi, PtrcallType};
use std::fmt;

pub struct RefArg<'r, T> {
    /// Only `None` if `T: GodotNullableFfi` and `T::is_null()` is true.
    shared_ref: Option<&'r T>,
}

impl<'r, T> RefArg<'r, T> {
    pub fn new(shared_ref: &'r T) -> Self {
        RefArg {
            shared_ref: Some(shared_ref),
        }
    }
}

macro_rules! wrong_direction {
    ($fn:ident) => {
        unreachable!(concat!(
            stringify!($fn),
            ": RefArg should only be passed *to* Godot, not *from*."
        ))
    };
}

impl<'r, T> GodotConvert for RefArg<'r, T>
where
    T: GodotConvert,
{
    type Via = T::Via;
}

impl<'r, T> ToGodot for RefArg<'r, T>
where
    T: ToGodot,
{
    type ToVia<'v> = T::ToVia<'v>
    where Self: 'v;

    fn to_godot(&self) -> Self::ToVia<'_> {
        let shared_ref = self
            .shared_ref
            .expect("Objects are currently mapped through ObjectArg; RefArg shouldn't be null");

        shared_ref.to_godot()
    }
}

// TODO refactor signature tuples into separate in+out traits, so FromGodot is no longer needed.
impl<'r, T> FromGodot for RefArg<'r, T>
where
    T: FromGodot,
{
    fn try_from_godot(_via: Self::Via) -> Result<Self, ConvertError> {
        wrong_direction!(try_from_godot)
    }
}

impl<'r, T> fmt::Debug for RefArg<'r, T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "&{:?}", self.shared_ref)
    }
}

// SAFETY: delegated to T.
unsafe impl<'r, T> GodotFfi for RefArg<'r, T>
where
    T: GodotFfi,
{
    fn variant_type() -> sys::VariantType {
        T::variant_type()
    }

    unsafe fn new_from_sys(_ptr: sys::GDExtensionConstTypePtr) -> Self {
        wrong_direction!(new_from_sys)
    }

    unsafe fn new_with_uninit(_init_fn: impl FnOnce(sys::GDExtensionUninitializedTypePtr)) -> Self {
        wrong_direction!(new_with_uninit)
    }

    unsafe fn new_with_init(_init_fn: impl FnOnce(sys::GDExtensionTypePtr)) -> Self {
        wrong_direction!(new_with_init)
    }

    fn sys(&self) -> sys::GDExtensionConstTypePtr {
        match self.shared_ref {
            Some(r) => r.sys(),
            None => std::ptr::null(),
        }
    }

    fn sys_mut(&mut self) -> sys::GDExtensionTypePtr {
        unreachable!("RefArg::sys_mut() currently not used by FFI marshalling layer, but only by specific functions");
    }

    // This function must be overridden; the default delegating to sys() is wrong for e.g. RawGd<T>.
    // See also other manual overrides of as_arg_ptr().
    fn as_arg_ptr(&self) -> sys::GDExtensionConstTypePtr {
        match self.shared_ref {
            Some(r) => r.as_arg_ptr(),
            None => std::ptr::null(),
        }
    }

    unsafe fn from_arg_ptr(_ptr: sys::GDExtensionTypePtr, _call_type: PtrcallType) -> Self {
        wrong_direction!(from_arg_ptr)
    }

    unsafe fn move_return_ptr(self, _dst: sys::GDExtensionTypePtr, _call_type: PtrcallType) {
        // This one is implemented, because it's used for return types implementing ToGodot.
        unreachable!("Calling RefArg::move_return_ptr is a mistake, as RefArg is intended only for arguments. Use the underlying value type.");
    }
}

impl<'r, T> GodotFfiVariant for RefArg<'r, T>
where
    T: GodotFfiVariant,
{
    fn ffi_to_variant(&self) -> Variant {
        match self.shared_ref {
            Some(r) => r.ffi_to_variant(),
            None => Variant::nil(),
        }
    }

    fn ffi_from_variant(_variant: &Variant) -> Result<Self, ConvertError> {
        wrong_direction!(ffi_from_variant)
    }
}

impl<'r, T> GodotNullableFfi for RefArg<'r, T>
where
    T: GodotNullableFfi,
{
    fn null() -> Self {
        RefArg { shared_ref: None }
    }

    fn is_null(&self) -> bool {
        self.shared_ref.map(|r| r.is_null()).unwrap_or(true)
    }
}
