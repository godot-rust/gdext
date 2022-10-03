/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::obj::InstanceId;
use gdext_builtin::{GodotString, Vector2, Vector3};
use gdext_sys as sys;
use std::fmt::Debug;

pub trait PropertyInfoBuilder {
    fn variant_type() -> sys::GDNativeVariantType;

    fn property_info(property_name: &str) -> sys::GDNativePropertyInfo {
        let reg = unsafe { sys::get_registry() };
        sys::GDNativePropertyInfo {
            type_: Self::variant_type() as u32,
            name: reg.c_string(property_name),
            class_name: std::ptr::null_mut(),
            hint: 0,
            hint_string: std::ptr::null_mut(),
            usage: 7, // Default, TODO generate global enums
        }
    }

    fn param_metadata() -> sys::GDNativeExtensionClassMethodArgumentMetadata {
        sys::GDNativeExtensionClassMethodArgumentMetadata_GDNATIVE_EXTENSION_METHOD_ARGUMENT_METADATA_NONE
    }
}

impl PropertyInfoBuilder for () {
    fn variant_type() -> sys::GDNativeVariantType {
        sys::GDNativeVariantType_GDNATIVE_VARIANT_TYPE_NIL
    }
}

impl PropertyInfoBuilder for bool {
    fn variant_type() -> sys::GDNativeVariantType {
        sys::GDNativeVariantType_GDNATIVE_VARIANT_TYPE_BOOL
    }
}

impl PropertyInfoBuilder for GodotString {
    fn variant_type() -> sys::GDNativeVariantType {
        sys::GDNativeVariantType_GDNATIVE_VARIANT_TYPE_STRING
    }
}

//
/*
impl PropertyInfoBuilder for &GodotString {
    fn variant_type() -> sys::GDNativeVariantType {
        sys::GDNativeVariantType_GDNATIVE_VARIANT_TYPE_STRING
    }
}
*/

impl PropertyInfoBuilder for Vector2 {
    fn variant_type() -> sys::GDNativeVariantType {
        sys::GDNativeVariantType_GDNATIVE_VARIANT_TYPE_VECTOR2
    }
}

impl PropertyInfoBuilder for Vector3 {
    fn variant_type() -> sys::GDNativeVariantType {
        sys::GDNativeVariantType_GDNATIVE_VARIANT_TYPE_VECTOR3
    }
}

macro_rules! property_info_integer {
    ($type:ty, $meta:ident) => {
        impl PropertyInfoBuilder for $type {
            fn variant_type() -> sys::GDNativeVariantType {
                sys::GDNativeVariantType_GDNATIVE_VARIANT_TYPE_INT
            }

            fn param_metadata() -> sys::GDNativeExtensionClassMethodArgumentMetadata {
                sys::$meta
            }
        }
    };
}

property_info_integer!(u8, GDNativeExtensionClassMethodArgumentMetadata_GDNATIVE_EXTENSION_METHOD_ARGUMENT_METADATA_INT_IS_UINT8);
property_info_integer!(u16, GDNativeExtensionClassMethodArgumentMetadata_GDNATIVE_EXTENSION_METHOD_ARGUMENT_METADATA_INT_IS_UINT16);
property_info_integer!(u32, GDNativeExtensionClassMethodArgumentMetadata_GDNATIVE_EXTENSION_METHOD_ARGUMENT_METADATA_INT_IS_UINT32);
property_info_integer!(u64, GDNativeExtensionClassMethodArgumentMetadata_GDNATIVE_EXTENSION_METHOD_ARGUMENT_METADATA_INT_IS_UINT64);

property_info_integer!(i8, GDNativeExtensionClassMethodArgumentMetadata_GDNATIVE_EXTENSION_METHOD_ARGUMENT_METADATA_INT_IS_INT8);
property_info_integer!(i16, GDNativeExtensionClassMethodArgumentMetadata_GDNATIVE_EXTENSION_METHOD_ARGUMENT_METADATA_INT_IS_INT16);
property_info_integer!(i32, GDNativeExtensionClassMethodArgumentMetadata_GDNATIVE_EXTENSION_METHOD_ARGUMENT_METADATA_INT_IS_INT32);
property_info_integer!(i64, GDNativeExtensionClassMethodArgumentMetadata_GDNATIVE_EXTENSION_METHOD_ARGUMENT_METADATA_INT_IS_INT64);

property_info_integer!(InstanceId, GDNativeExtensionClassMethodArgumentMetadata_GDNATIVE_EXTENSION_METHOD_ARGUMENT_METADATA_INT_IS_INT64);

// ----------------------------------------------------------------------------------------------------------------------------------------------

pub trait SignatureTuple {
    type Params;
    type Ret;

    fn variant_type(index: i32) -> sys::GDNativeVariantType;
    fn param_metadata(index: i32) -> sys::GDNativeExtensionClassMethodArgumentMetadata;
    fn property_info(index: i32, param_name: &str) -> sys::GDNativePropertyInfo;

    fn varcall<C: GodotClass>(
        instance_ptr: sys::GDExtensionClassInstancePtr,
        args_ptr: *const sys::GDNativeVariantPtr,
        ret: sys::GDNativeVariantPtr,
        err: *mut sys::GDNativeCallError,
        func: fn(&mut C, Self::Params) -> Self::Ret,
        method_name: &str,
    );

    // Note: this method imposes extra bounds on GodotFfi, which may not be implemented for user types.
    // We could fall back to varcalls in such cases, and not require GodotFfi categorically.
    fn ptrcall<C: GodotClass>(
        instance_ptr: sys::GDExtensionClassInstancePtr,
        args_ptr: *const sys::GDNativeTypePtr,
        ret: sys::GDNativeTypePtr,
        func: fn(&mut C, Self::Params) -> Self::Ret,
        method_name: &str,
    );
}

// impl<P, const N: usize> Sig for [P; N]
// impl<P, T0> Sig for (T0)
// where P: PropertyInfoBuilder {
//     fn variant_type(index: usize) -> sys::GDNativeVariantType {
//           Self[index]::
//     }
//
//     fn param_metadata(index: usize) -> sys::GDNativeExtensionClassMethodArgumentMetadata {
//         todo!()
//     }
//
//     fn property_info(index: usize, param_name: &str) -> sys::GDNativePropertyInfo {
//         todo!()
//     }
// }
//
use crate::traits::GodotClass;
use gdext_builtin::{FromVariant, ToVariant, Variant};

macro_rules! impl_signature_for_tuple {
    (
        $R:ident
        $(, $Pn:ident : $n:literal)*
    ) => {
        #[allow(unused_variables)]
        impl<$R, $($Pn,)*> SignatureTuple for ($R, $($Pn,)*)
            where $R: PropertyInfoBuilder + ToVariant + sys::GodotFuncMarshal + Debug,
               $( $Pn: PropertyInfoBuilder + FromVariant + sys::GodotFuncMarshal + Debug, )*
        {
            type Params = ($($Pn,)*);
            type Ret = $R;

            #[inline]
            fn variant_type(index: i32) -> sys::GDNativeVariantType {
                match index {
                    -1 => $R::variant_type(),
                    $(
                        $n => $Pn::variant_type(),
                    )*
                    _ => unreachable!("variant_type: unavailable for index {}", index),
                    //_ => sys::GDNativeVariantType_GDNATIVE_VARIANT_TYPE_NIL
                }
            }

            #[inline]
            fn param_metadata(index: i32) -> sys::GDNativeExtensionClassMethodArgumentMetadata {
                match index {
                    -1 => $R::param_metadata(),
                    $(
                        $n => $Pn::param_metadata(),
                    )*
                    _ => unreachable!("param_metadata: unavailable for index {}", index),
                }
            }

            #[inline]
            fn property_info(index: i32, param_name: &str) -> sys::GDNativePropertyInfo {
                match index {
                    -1 => $R::property_info(param_name),
                    $(
                        $n => $Pn::property_info(param_name),
                    )*
                    _ => unreachable!("property_info: unavailable for index {}", index),
                }
            }

            #[inline]
            fn varcall<C : GodotClass>(
				instance_ptr: sys::GDExtensionClassInstancePtr,
                args_ptr: *const sys::GDNativeVariantPtr,
                ret: sys::GDNativeVariantPtr,
                err: *mut sys::GDNativeCallError,
                func: fn(&mut C, Self::Params) -> Self::Ret,
                method_name: &str,
            ) {
    	        println!("varcall: {}", method_name);

                let storage = unsafe { crate::private::as_storage::<C>(instance_ptr) };
                let mut instance = storage.get_mut();

                let args = ( $(
                    {
                        let variant = unsafe { &*(*args_ptr.offset($n) as *mut Variant) }; // TODO from_var_sys
                        let arg = <$Pn as FromVariant>::try_from_variant(variant)
                            .unwrap_or_else(|e| param_error::<$Pn>(method_name, $n, variant));

                        arg
                    },
                )* );

				let ret_val = func(&mut *instance, args);
                let ret_variant = <$R as ToVariant>::to_variant(&ret_val); // TODO write_sys
				unsafe {
                    *(ret as *mut Variant) = ret_variant;
                    (*err).error = sys::GDNativeCallErrorType_GDNATIVE_CALL_OK;
                }
            }

            #[inline]
            fn ptrcall<C : GodotClass>(
				instance_ptr: sys::GDExtensionClassInstancePtr,
                args_ptr: *const sys::GDNativeTypePtr,
                ret: sys::GDNativeTypePtr,
                func: fn(&mut C, Self::Params) -> Self::Ret,
                method_name: &str,
            ) {
                println!("ptrcall: {}", method_name);
                let storage = unsafe { crate::private::as_storage::<C>(instance_ptr) };
                let mut instance = storage.get_mut();

				let args = ( $(
                    unsafe { <$Pn as sys::GodotFuncMarshal>::try_from_sys(*args_ptr.offset($n)) }
                        .unwrap_or_else(|e| param_error::<$Pn>(method_name, $n, &e)),
                )* );

                let ret_val = func(&mut *instance, args);
				unsafe { <$R as sys::GodotFuncMarshal>::try_write_sys(&ret_val, ret) }
                    .unwrap_or_else(|e| return_error::<$R>(method_name, &e));

                // FIXME should be inc_ref instead of forget
				std::mem::forget(ret_val);
            }
        }
    };
}

fn param_error<P>(method_name: &str, index: i32, arg: &impl Debug) -> ! {
    let param_ty = std::any::type_name::<P>();
    panic!(
        "{method_name}: parameter [{index}] has type {param_ty}, which is unable to store argument {arg:?}",
    );
}

fn return_error<R>(method_name: &str, arg: &impl Debug) -> ! {
    let return_ty = std::any::type_name::<R>();
    panic!(
        "{method_name}: return type {return_ty} is unable to store value {arg:?}",
    );
}

impl_signature_for_tuple!(R);
impl_signature_for_tuple!(R, P0: 0);
impl_signature_for_tuple!(R, P0: 0, P1: 1);
impl_signature_for_tuple!(R, P0: 0, P1: 1, P2: 2);
impl_signature_for_tuple!(R, P0: 0, P1: 1, P2: 2, P3: 3);
impl_signature_for_tuple!(R, P0: 0, P1: 1, P2: 2, P3: 3, P4: 4);
impl_signature_for_tuple!(R, P0: 0, P1: 1, P2: 2, P3: 3, P4: 4, P5: 5);
impl_signature_for_tuple!(R, P0: 0, P1: 1, P2: 2, P3: 3, P4: 4, P5: 5, P6: 6);
impl_signature_for_tuple!(R, P0: 0, P1: 1, P2: 2, P3: 3, P4: 4, P5: 5, P6: 6, P7: 7);
impl_signature_for_tuple!(R, P0: 0, P1: 1, P2: 2, P3: 3, P4: 4, P5: 5, P6: 6, P7: 7, P8: 8);
impl_signature_for_tuple!(R, P0: 0, P1: 1, P2: 2, P3: 3, P4: 4, P5: 5, P6: 6, P7: 7, P8: 8, P9: 9);

// Re-exported to crate::private
#[doc(hidden)]
pub mod func_callbacks {
    use super::*;

    pub extern "C" fn get_type<S: SignatureTuple>(
        _method_data: *mut std::ffi::c_void,
        n: i32,
    ) -> sys::GDNativeVariantType {
        S::variant_type(n)
    }

    pub extern "C" fn get_info<S: SignatureTuple>(
        _method_data: *mut std::ffi::c_void,
        n: i32,
        ret: *mut sys::GDNativePropertyInfo,
    ) {
        // Return value is the first "argument"
        let info = S::property_info(n, "TODO");
        unsafe { *ret = info };
    }

    pub extern "C" fn get_metadata<S: SignatureTuple>(
        _method_data: *mut std::ffi::c_void,
        n: i32,
    ) -> sys::GDNativeExtensionClassMethodArgumentMetadata {
        // Return value is the first "argument"
        S::param_metadata(n)
    }
}
