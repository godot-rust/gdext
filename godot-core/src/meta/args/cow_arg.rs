/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::fmt;
use std::ops::Deref;

use godot_ffi::{ExtVariantType, GodotFfi, GodotNullableFfi, PtrcallType};

use crate::builtin::Variant;
use crate::meta::error::ConvertError;
use crate::meta::{GodotConvert, GodotFfiVariant, ObjectArg, RefArg, ToGodot};
use crate::sys;

/// FFI-optimized argument. Like `CowArg`, but with additional "short-circuit" path to pass objects to FFI.
#[doc(hidden)]
#[derive(PartialEq)]
pub enum FfiArg<'arg, T> {
    Cow(CowArg<'arg, T>),
    FfiObject(ObjectArg<'arg>),
}

/// Owned or borrowed value, used when passing arguments through `impl AsArg` to Godot APIs.
#[doc(hidden)]
#[derive(PartialEq)]
pub enum CowArg<'arg, T> {
    Owned(T),
    Borrowed(&'arg T),
}

impl<T> CowArg<'_, T> {
    pub fn cow_into_owned(self) -> T
    where
        T: Clone,
    {
        match self {
            CowArg::Owned(v) => v,
            CowArg::Borrowed(r) => r.clone(),
        }
    }

    pub fn cow_as_ref(&self) -> &T {
        match self {
            CowArg::Owned(v) => v,
            CowArg::Borrowed(r) => r,
        }
    }

    /// Returns the actual argument to be passed to function calls.
    ///
    /// [`CowArg`] does not implement [`AsArg<T>`] because a differently-named method is more explicit (fewer errors in codegen),
    /// and because [`AsArg::into_arg()`] is not meaningful.
    pub fn cow_as_arg(&self) -> RefArg<'_, T> {
        RefArg::new(self.cow_as_ref())
    }
}

macro_rules! wrong_direction {
    ($fn:ident) => {
        unreachable!(concat!(
            stringify!($fn),
            ": CowArg should only be passed *to* Godot, not *from*."
        ))
    };
}

impl<T> GodotConvert for CowArg<'_, T>
where
    T: GodotConvert,
{
    type Via = T::Via;
}

impl<T> ToGodot for CowArg<'_, T>
where
    T: ToGodot,
{
    type Pass = T::Pass;

    fn to_godot(&self) -> crate::meta::ToArg<'_, Self::Via, Self::Pass> {
        // Forward to the wrapped type's to_godot implementation
        self.cow_as_ref().to_godot()
    }

    fn to_godot_owned(&self) -> Self::Via
    where
        Self::Via: Clone,
    {
        // Default implementation calls underlying T::to_godot().clone(), which is wrong.
        // Some to_godot_owned() calls are specialized/overridden, we need to honor that.

        self.cow_as_ref().to_godot_owned()
    }
}

impl<T> fmt::Debug for CowArg<'_, T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CowArg::Owned(v) => write!(f, "CowArg::Owned({v:?})"),
            CowArg::Borrowed(r) => write!(f, "CowArg::Borrowed({r:?})"),
        }
    }
}

impl<T> GodotNullableFfi for CowArg<'_, T>
where
    T: GodotNullableFfi,
{
    fn null() -> Self {
        CowArg::Owned(T::null())
    }

    fn is_null(&self) -> bool {
        self.cow_as_ref().is_null()
    }
}

impl<T> Deref for CowArg<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self {
            CowArg::Owned(value) => value,
            CowArg::Borrowed(value) => value,
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// FfiArg implementations

impl<T> GodotNullableFfi for FfiArg<'_, T>
where
    T: GodotNullableFfi,
{
    fn null() -> Self {
        FfiArg::Cow(CowArg::Owned(T::null()))
    }

    fn is_null(&self) -> bool {
        match self {
            FfiArg::Cow(cow_arg) => cow_arg.is_null(),
            FfiArg::FfiObject(obj_arg) => obj_arg.is_null(),
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Macro to implement similar trait impls between [`CowArg`] and [`FfiArg`].
///
/// Debug and null constructors are implemented manually since they're distinct enough.
macro_rules! impl_ffi_traits {
    ($ArgType:ident {
        $($enum_pattern:pat => $delegate:expr),* $(,)?
    }) => {
        // SAFETY: delegated to inner values.
        unsafe impl<T> GodotFfi for $ArgType<'_, T>
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
                match self {
                    $($enum_pattern => $delegate.sys(),)*
                }
            }

            fn sys_mut(&mut self) -> sys::GDExtensionTypePtr {
                unreachable!(concat!(stringify!($ArgType), "::sys_mut() currently not used by FFI marshalling layer, but only by specific functions"));
            }

            fn as_arg_ptr(&self) -> sys::GDExtensionConstTypePtr {
                match self {
                    $($enum_pattern => $delegate.as_arg_ptr(),)*
                }
            }

            unsafe fn from_arg_ptr(_ptr: sys::GDExtensionTypePtr, _call_type: PtrcallType) -> Self {
                wrong_direction!(from_arg_ptr)
            }

            unsafe fn move_return_ptr(self, _dst: sys::GDExtensionTypePtr, _call_type: PtrcallType) {
                unreachable!(concat!("Calling ", stringify!($ArgType), "::move_return_ptr is a mistake, as ", stringify!($ArgType), " is intended only for arguments. Use the underlying value type."));
            }
        }

        impl<T> GodotFfiVariant for $ArgType<'_, T>
        where
            T: GodotFfiVariant,
        {
            fn ffi_to_variant(&self) -> Variant {
                match self {
                    $($enum_pattern => $delegate.ffi_to_variant(),)*
                }
            }

            fn ffi_from_variant(_variant: &Variant) -> Result<Self, ConvertError> {
                wrong_direction!(ffi_from_variant)
            }
        }
    };
}

impl_ffi_traits! {
    CowArg {
        CowArg::Owned(v) => v,
        CowArg::Borrowed(r) => *r,
    }
}

impl_ffi_traits! {
    FfiArg {
        FfiArg::Cow(cow_arg) => cow_arg,
        FfiArg::FfiObject(obj_arg) => obj_arg,
    }
}
