/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::obj::{Gd, GodotClass};
use crate::sys;

use super::{ConvertError, FromGodot, GodotType};

/// Specifies how the return type is marshalled in a ptrcall.
#[doc(hidden)]
pub trait PtrcallReturn {
    type Ret;

    unsafe fn call(
        process_return_ptr: impl FnMut(sys::GDExtensionTypePtr),
    ) -> Result<Self::Ret, ConvertError>;
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[doc(hidden)]
pub struct PtrcallReturnOptionGdT<R> {
    _marker: std::marker::PhantomData<R>,
}

impl<T: GodotClass> PtrcallReturn for PtrcallReturnOptionGdT<Gd<T>> {
    type Ret = Option<Gd<T>>;

    unsafe fn call(
        process_return_ptr: impl FnMut(sys::GDExtensionTypePtr),
    ) -> Result<Self::Ret, ConvertError> {
        Ok(Gd::<T>::from_sys_init_opt(process_return_ptr))
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[doc(hidden)]
pub struct PtrcallReturnT<R> {
    _marker: std::marker::PhantomData<R>,
}

impl<T: FromGodot> PtrcallReturn for PtrcallReturnT<T> {
    type Ret = T;

    unsafe fn call(
        mut process_return_ptr: impl FnMut(sys::GDExtensionTypePtr),
    ) -> Result<Self::Ret, ConvertError> {
        let ffi =
            <<T::Via as GodotType>::Ffi as sys::GodotFfi>::from_sys_init_default(|return_ptr| {
                process_return_ptr(return_ptr)
            });

        T::Via::try_from_ffi(ffi).and_then(T::try_from_godot)
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[doc(hidden)]
pub enum PtrcallReturnUnit {}

impl PtrcallReturn for PtrcallReturnUnit {
    type Ret = ();

    unsafe fn call(
        mut process_return_ptr: impl FnMut(sys::GDExtensionTypePtr),
    ) -> Result<Self::Ret, ConvertError> {
        let return_ptr = std::ptr::null_mut();
        process_return_ptr(return_ptr);
        Ok(())
    }
}
