/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot_ffi as sys;

use crate::builtin::{inner, FromVariant, Variant};
use std::marker::PhantomData;
use sys::types::*;
use sys::{ffi_methods, interface_fn, GodotFfi};

impl_builtin_stub!(Array, OpaqueArray);
impl_builtin_stub!(PackedByteArray, OpaquePackedByteArray);
impl_builtin_stub!(PackedColorArray, OpaquePackedColorArray);
impl_builtin_stub!(PackedFloat32Array, OpaquePackedFloat32Array);
impl_builtin_stub!(PackedFloat64Array, OpaquePackedFloat64Array);
impl_builtin_stub!(PackedInt32Array, OpaquePackedInt32Array);
impl_builtin_stub!(PackedInt64Array, OpaquePackedInt64Array);
impl_builtin_stub!(PackedStringArray, OpaquePackedStringArray);
impl_builtin_stub!(PackedVector2Array, OpaquePackedVector2Array);
impl_builtin_stub!(PackedVector3Array, OpaquePackedVector3Array);

impl_builtin_froms!(Array;
    PackedByteArray => array_from_packed_byte_array,
    PackedColorArray => array_from_packed_color_array,
    PackedFloat32Array => array_from_packed_float32_array,
    PackedFloat64Array => array_from_packed_float64_array,
    PackedInt32Array => array_from_packed_int32_array,
    PackedInt64Array => array_from_packed_int64_array,
    PackedStringArray => array_from_packed_string_array,
    PackedVector2Array => array_from_packed_vector2_array,
    PackedVector3Array => array_from_packed_vector3_array,
);

impl_builtin_froms!(PackedByteArray; Array => packed_byte_array_from_array);
impl_builtin_froms!(PackedColorArray; Array => packed_color_array_from_array);
impl_builtin_froms!(PackedFloat32Array; Array => packed_float32_array_from_array);
impl_builtin_froms!(PackedFloat64Array; Array => packed_float64_array_from_array);
impl_builtin_froms!(PackedInt32Array; Array => packed_int32_array_from_array);
impl_builtin_froms!(PackedInt64Array; Array => packed_int64_array_from_array);
impl_builtin_froms!(PackedStringArray; Array => packed_string_array_from_array);
impl_builtin_froms!(PackedVector2Array; Array => packed_vector2_array_from_array);
impl_builtin_froms!(PackedVector3Array; Array => packed_vector3_array_from_array);

impl Array {
    pub fn get(&self, index: i64) -> Option<Variant> {
        unsafe {
            let ptr = (interface_fn!(array_operator_index))(self.sys(), index) as *mut Variant;
            if ptr.is_null() {
                return None;
            }
            Some((*ptr).clone())
        }
    }

    #[cfg(not(any(gdext_test, doctest)))]
    #[doc(hidden)]
    pub fn as_inner(&mut self) -> inner::InnerArray {
        inner::InnerArray { outer: self }
    }
}

impl_builtin_traits! {
    for Array {
        Default => array_construct_default;
        Clone => array_construct_copy;
        Drop => array_destroy;
    }
}

#[repr(C)]
pub struct TypedArray<T> {
    opaque: OpaqueArray,
    _phantom: PhantomData<T>,
}
impl<T> TypedArray<T> {
    fn from_opaque(opaque: OpaqueArray) -> Self {
        Self {
            opaque,
            _phantom: PhantomData,
        }
    }
}

impl<T> Clone for TypedArray<T> {
    fn clone(&self) -> Self {
        unsafe {
            Self::from_sys_init(|self_ptr| {
                let ctor = ::godot_ffi::builtin_fn!(array_construct_copy);
                let args = [self.sys_const()];
                ctor(self_ptr, args.as_ptr());
            })
        }
    }
}

// TODO enable this:
// impl_builtin_traits! {
//     for TypedArray<T> {
//         Clone => array_construct_copy;
//         Drop => array_destroy;
//     }
// }

impl<T> GodotFfi for TypedArray<T> {
    ffi_methods! { type sys::GDExtensionTypePtr = *mut Opaque; .. }
}

impl<T> Drop for TypedArray<T> {
    fn drop(&mut self) {
        unsafe {
            let destructor = sys::builtin_fn!(array_destroy @1);
            destructor(self.sys_mut());
        }
    }
}

impl<T: FromVariant> TypedArray<T> {
    pub fn get(&self, index: i64) -> Option<T> {
        unsafe {
            let ptr = (interface_fn!(array_operator_index))(self.sys(), index);
            let v = Variant::from_var_sys(ptr);
            T::try_from_variant(&v).ok()
        }
    }
}
