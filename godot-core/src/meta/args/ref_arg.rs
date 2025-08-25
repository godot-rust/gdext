/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::fmt;

use godot_ffi::{ExtVariantType, GodotFfi, GodotNullableFfi, PtrcallType};

use crate::builtin::Variant;
use crate::meta::error::ConvertError;
use crate::meta::{FromGodot, GodotConvert, GodotFfiVariant, ToGodot};
use crate::sys;

/// Simple reference wrapper, used when passing arguments by-ref to Godot APIs.
///
/// This type is often used as the result of [`ToGodot::to_godot()`], if `Self` is not a `Copy` type.
pub struct RefArg<'r, T> {
    /// Only `None` if `T: GodotNullableFfi` and `T::is_null()` is true.
    shared_ref: Option<&'r T>,
}

impl<'r, T> RefArg<'r, T> {
    /// Creates a new `RefArg` from a reference.
    ///
    /// Unless you implement your own `ToGodot` impl, there is usually no reason to use this.
    pub fn new(shared_ref: &'r T) -> Self {
        RefArg {
            shared_ref: Some(shared_ref),
        }
    }

    // Note: the following APIs are not used by gdext itself, but exist for user convenience, since
    // RefArg is a public type returned by ToGodot::to_godot(). Does not implementing AsRef + ToOwned, because `RefArg` is intended
    // to be a niche API, not common occurrence in user code. ToOwned would also impose Borrow<T> on other types.

    /// Returns the stored reference.
    ///
    /// # Panics
    /// If `T` is `Option<Gd<...>>::None`.
    pub fn get_ref(&self) -> &T {
        self.shared_ref.expect("RefArg is null")
    }

    /// Returns the stored reference.
    ///
    /// Returns `None` if `T` is `Option<Gd<...>>::None`.
    pub fn get_ref_or_none(&self) -> Option<&T>
    where
        T: GodotNullableFfi,
    {
        self.shared_ref
    }

    /// Returns the stored reference.
    ///
    /// # Panics
    /// If `T` is `Option<Gd<...>>::None`.
    pub fn to_owned(&self) -> T
    where
        T: Clone,
    {
        self.get_ref().clone()
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

impl<T> GodotConvert for RefArg<'_, T>
where
    T: GodotConvert,
{
    type Via = T::Via;
}

impl<T> ToGodot for RefArg<'_, T>
where
    T: ToGodot,
{
    type Pass = T::Pass;

    fn to_godot(&self) -> crate::meta::ToArg<'_, Self::Via, Self::Pass> {
        let shared_ref = self
            .shared_ref
            .expect("Objects are currently mapped through ObjectArg; RefArg shouldn't be null");

        shared_ref.to_godot()
    }

    fn to_godot_owned(&self) -> Self::Via
    where
        Self::Via: Clone,
    {
        // Default implementation calls underlying T::to_godot().clone(), which is wrong.
        // Some to_godot_owned() calls are specialized/overridden, we need to honor that.

        let shared_ref = self
            .shared_ref
            .expect("Objects are currently mapped through ObjectArg; RefArg shouldn't be null");

        shared_ref.to_godot_owned()
    }
}

// TODO refactor signature tuples into separate in+out traits, so FromGodot is no longer needed.
impl<T> FromGodot for RefArg<'_, T>
where
    T: FromGodot,
{
    fn try_from_godot(_via: Self::Via) -> Result<Self, ConvertError> {
        wrong_direction!(try_from_godot)
    }
}

impl<T> fmt::Debug for RefArg<'_, T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "&{:?}", self.shared_ref)
    }
}

// SAFETY: delegated to T.
unsafe impl<T> GodotFfi for RefArg<'_, T>
where
    T: GodotFfi,
{
    const VARIANT_TYPE: ExtVariantType = T::VARIANT_TYPE;

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

impl<T> GodotFfiVariant for RefArg<'_, T>
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

impl<T> GodotNullableFfi for RefArg<'_, T>
where
    T: GodotNullableFfi,
{
    fn null() -> Self {
        RefArg { shared_ref: None }
    }

    fn is_null(&self) -> bool {
        self.shared_ref.is_none_or(T::is_null)
    }
}
