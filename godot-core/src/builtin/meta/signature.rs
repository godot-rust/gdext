/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot_ffi as sys;
use godot_ffi::VariantType;
use std::fmt::Debug;

#[doc(hidden)]
pub trait SignatureTuple {
    type Params;
    type Ret;

    fn variant_type(index: i32) -> VariantType;
    fn property_info(index: i32, param_name: &str) -> PropertyInfo;
    fn param_metadata(index: i32) -> sys::GDExtensionClassMethodArgumentMetadata;

    unsafe fn varcall<C: GodotClass>(
        instance_ptr: sys::GDExtensionClassInstancePtr,
        args_ptr: *const sys::GDExtensionConstVariantPtr,
        ret: sys::GDExtensionVariantPtr,
        err: *mut sys::GDExtensionCallError,
        func: fn(&mut C, Self::Params) -> Self::Ret,
        method_name: &str,
    );

    // Note: this method imposes extra bounds on GodotFfi, which may not be implemented for user types.
    // We could fall back to varcalls in such cases, and not require GodotFfi categorically.
    unsafe fn ptrcall<C: GodotClass>(
        instance_ptr: sys::GDExtensionClassInstancePtr,
        args_ptr: *const sys::GDExtensionConstTypePtr,
        ret: sys::GDExtensionTypePtr,
        func: fn(&mut C, Self::Params) -> Self::Ret,
        method_name: &str,
    );
}

// impl<P, const N: usize> Sig for [P; N]
// impl<P, T0> Sig for (T0)
// where P: VariantMetadata {
//     fn variant_type(index: usize) -> sys::GDExtensionVariantType {
//           Self[index]::
//     }
//
//     fn param_metadata(index: usize) -> sys::GDExtensionClassMethodArgumentMetadata {
//         todo!()
//     }
//
//     fn property_info(index: usize, param_name: &str) -> sys::GDExtensionPropertyInfo {
//         todo!()
//     }
// }
//
use crate::builtin::meta::*;
use crate::builtin::{FromVariant, ToVariant, Variant};
use crate::obj::GodotClass;

macro_rules! impl_signature_for_tuple {
    (
        $R:ident
        $(, $Pn:ident : $n:literal)*
    ) => {
        #[allow(unused_variables)]
        impl<$R, $($Pn,)*> SignatureTuple for ($R, $($Pn,)*)
            where $R: VariantMetadata + ToVariant + sys::GodotFuncMarshal + Debug,
               $( $Pn: VariantMetadata + FromVariant + sys::GodotFuncMarshal + Debug, )*
        {
            type Params = ($($Pn,)*);
            type Ret = $R;

            #[inline]
            fn variant_type(index: i32) -> sys::VariantType {
                match index {
                    -1 => $R::variant_type(),
                    $(
                        $n => $Pn::variant_type(),
                    )*
                    _ => unreachable!("variant_type: unavailable for index {}", index),
                    //_ => sys::GDEXTENSION_VARIANT_TYPE_NIL
                }
            }

            #[inline]
            fn param_metadata(index: i32) -> sys::GDExtensionClassMethodArgumentMetadata {
                match index {
                    -1 => $R::param_metadata(),
                    $(
                        $n => $Pn::param_metadata(),
                    )*
                    _ => unreachable!("param_metadata: unavailable for index {}", index),
                }
            }

            #[inline]
            fn property_info(index: i32, param_name: &str) -> PropertyInfo {
                match index {
                    -1 => $R::property_info(param_name),
                    $(
                        $n => $Pn::property_info(param_name),
                    )*
                    _ => unreachable!("property_info: unavailable for index {}", index),
                }
            }

            #[inline]
            unsafe fn varcall<C : GodotClass>(
				instance_ptr: sys::GDExtensionClassInstancePtr,
                args_ptr: *const sys::GDExtensionConstVariantPtr,
                ret: sys::GDExtensionVariantPtr,
                err: *mut sys::GDExtensionCallError,
                func: fn(&mut C, Self::Params) -> Self::Ret,
                method_name: &str,
            ) {
    	        $crate::out!("varcall: {}", method_name);

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
                    (*err).error = sys::GDEXTENSION_CALL_OK;
                }
            }

            #[inline]
            unsafe fn ptrcall<C : GodotClass>(
				instance_ptr: sys::GDExtensionClassInstancePtr,
                args_ptr: *const sys::GDExtensionConstTypePtr,
                ret: sys::GDExtensionTypePtr,
                func: fn(&mut C, Self::Params) -> Self::Ret,
                method_name: &str,
            ) {
                $crate::out!("ptrcall: {}", method_name);

                let storage = unsafe { crate::private::as_storage::<C>(instance_ptr) };
                let mut instance = storage.get_mut();

				let args = ( $(
                    unsafe {
                        <$Pn as sys::GodotFuncMarshal>::try_from_sys(
                            sys::force_mut_ptr(*args_ptr.offset($n))
                        )
                    }
                        .unwrap_or_else(|e| param_error::<$Pn>(method_name, $n, &e)),
                )* );

                let ret_val = func(&mut *instance, args);
				unsafe { <$R as sys::GodotFuncMarshal>::try_write_sys(&ret_val, ret) }
                    .unwrap_or_else(|e| return_error::<$R>(method_name, &e));

                // FIXME is inc_ref needed here?
				// std::mem::forget(ret_val);
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
    panic!("{method_name}: return type {return_ty} is unable to store value {arg:?}",);
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
