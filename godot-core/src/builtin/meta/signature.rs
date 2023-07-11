/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot_ffi as sys;
use std::fmt::Debug;

#[doc(hidden)]
pub trait VarcallSignatureTuple: PtrcallSignatureTuple {
    const PARAM_COUNT: usize;

    fn param_property_info(index: usize, param_name: &str) -> PropertyInfo;
    fn param_info(index: usize, param_name: &str) -> Option<MethodParamOrReturnInfo>;
    fn return_info() -> Option<MethodParamOrReturnInfo>;

    // TODO(uninit) - can we use this for varcall/ptrcall?
    // ret: sys::GDExtensionUninitializedVariantPtr
    // ret: sys::GDExtensionUninitializedTypePtr
    unsafe fn varcall(
        instance_ptr: sys::GDExtensionClassInstancePtr,
        args_ptr: *const sys::GDExtensionConstVariantPtr,
        ret: sys::GDExtensionVariantPtr,
        err: *mut sys::GDExtensionCallError,
        func: fn(sys::GDExtensionClassInstancePtr, Self::Params) -> Self::Ret,
        method_name: &str,
    );
}

#[doc(hidden)]
pub trait PtrcallSignatureTuple {
    type Params;
    type Ret;

    // Note: this method imposes extra bounds on GodotFfi, which may not be implemented for user types.
    // We could fall back to varcalls in such cases, and not require GodotFfi categorically.
    unsafe fn ptrcall(
        instance_ptr: sys::GDExtensionClassInstancePtr,
        args_ptr: *const sys::GDExtensionConstTypePtr,
        ret: sys::GDExtensionTypePtr,
        func: fn(sys::GDExtensionClassInstancePtr, Self::Params) -> Self::Ret,
        method_name: &str,
        call_type: sys::PtrcallType,
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

use super::registration::method::MethodParamOrReturnInfo;

macro_rules! impl_varcall_signature_for_tuple {
    (
        $PARAM_COUNT:literal,
        $R:ident
        $(, $Pn:ident : $n:literal)*
    ) => {
        #[allow(unused_variables)]
        impl<$R, $($Pn,)*> VarcallSignatureTuple for ($R, $($Pn,)*)
            where $R: VariantMetadata + ToVariant + sys::GodotFuncMarshal + Debug,
               $( $Pn: VariantMetadata + FromVariant + sys::GodotFuncMarshal + Debug, )*
        {
            const PARAM_COUNT: usize = $PARAM_COUNT;

            fn param_info(index: usize, param_name: &str) -> Option<MethodParamOrReturnInfo> {
                match index {
                    $(
                        $n => Some($Pn::argument_info(param_name)),
                    )*
                    _ => None,
                }
            }

            fn return_info() -> Option<MethodParamOrReturnInfo> {
                $R::return_info()
            }

            fn param_property_info(index: usize, param_name: &str) -> PropertyInfo {
                match index {
                    $(
                        $n => $Pn::property_info(param_name),
                    )*
                    _ => unreachable!("property_info: unavailable for index {}", index),
                }
            }


            #[inline]
            unsafe fn varcall(
                instance_ptr: sys::GDExtensionClassInstancePtr,
                args_ptr: *const sys::GDExtensionConstVariantPtr,
                ret: sys::GDExtensionVariantPtr,
                err: *mut sys::GDExtensionCallError,
                func: fn(sys::GDExtensionClassInstancePtr, Self::Params) -> Self::Ret,
                method_name: &str,
            ) {
                $crate::out!("varcall: {}", method_name);

                let args = ($(
                    unsafe { varcall_arg::<$Pn, $n>(args_ptr, method_name) },
                )*) ;

                varcall_return::<$R>(func(instance_ptr, args), ret, err)
            }
        }
    };
}

macro_rules! impl_ptrcall_signature_for_tuple {
    (
        $R:ident
        $(, $Pn:ident : $n:literal)*
    ) => {
        #[allow(unused_variables)]
        impl<$R, $($Pn,)*> PtrcallSignatureTuple for ($R, $($Pn,)*)
            where $R: sys::GodotFuncMarshal + Debug,
               $( $Pn: sys::GodotFuncMarshal + Debug, )*
        {
            type Params = ($($Pn,)*);
            type Ret = $R;

            unsafe fn ptrcall(
                instance_ptr: sys::GDExtensionClassInstancePtr,
                args_ptr: *const sys::GDExtensionConstTypePtr,
                ret: sys::GDExtensionTypePtr,
                func: fn(sys::GDExtensionClassInstancePtr, Self::Params) -> Self::Ret,
                method_name: &str,
                call_type: sys::PtrcallType,
            ) {
                $crate::out!("ptrcall: {}", method_name);

                let args = ($(
                    unsafe { ptrcall_arg::<$Pn, $n>(args_ptr, method_name, call_type) },
                )*) ;

                // SAFETY:
                // `ret` is always a pointer to an initialized value of type $R
                // TODO: double-check the above
                ptrcall_return::<$R>(func(instance_ptr, args), ret, method_name, call_type)
            }
        }
    };
}

/// Convert the `N`th argument of `args_ptr` into a value of type `P`.
///
/// # Safety
/// - It must be safe to dereference the pointer at `args_ptr.offset(N)` .
unsafe fn varcall_arg<P: FromVariant, const N: isize>(
    args_ptr: *const sys::GDExtensionConstVariantPtr,
    method_name: &str,
) -> P {
    let variant = &*(*args_ptr.offset(N) as *mut Variant); // TODO from_var_sys
    P::try_from_variant(variant)
        .unwrap_or_else(|_| param_error::<P>(method_name, N as i32, variant))
}

/// Moves `ret_val` into `ret`.
///
/// # Safety
/// - `ret` must be a pointer to an initialized `Variant`.
/// - It must be safe to write a `Variant` once to `ret`.
/// - It must be safe to write a `sys::GDExtensionCallError` once to `err`.
unsafe fn varcall_return<R: ToVariant>(
    ret_val: R,
    ret: sys::GDExtensionVariantPtr,
    err: *mut sys::GDExtensionCallError,
) {
    let ret_variant = ret_val.to_variant(); // TODO write_sys
    *(ret as *mut Variant) = ret_variant;
    (*err).error = sys::GDEXTENSION_CALL_OK;
}

/// Convert the `N`th argument of `args_ptr` into a value of type `P`.
///
/// # Safety
/// - It must be safe to dereference the address at `args_ptr.offset(N)` .
/// - The pointer at `args_ptr.offset(N)` must follow the safety requirements as laid out in
///   [`GodotFuncMarshal::try_from_arg`][sys::GodotFuncMarshal::try_from_arg].
unsafe fn ptrcall_arg<P: sys::GodotFuncMarshal, const N: isize>(
    args_ptr: *const sys::GDExtensionConstTypePtr,
    method_name: &str,
    call_type: sys::PtrcallType,
) -> P {
    P::try_from_arg(sys::force_mut_ptr(*args_ptr.offset(N)), call_type)
        .unwrap_or_else(|e| param_error::<P>(method_name, N as i32, &e))
}

/// Moves `ret_val` into `ret`.
///
/// # Safety
/// `ret_val`, `ret`, and `call_type` must follow the safety requirements as laid out in
/// [`GodotFuncMarshal::try_return`](sys::GodotFuncMarshal::try_return).
unsafe fn ptrcall_return<R: sys::GodotFuncMarshal + std::fmt::Debug>(
    ret_val: R,
    ret: sys::GDExtensionTypePtr,
    method_name: &str,
    call_type: sys::PtrcallType,
) {
    ret_val
        .try_return(ret, call_type)
        .unwrap_or_else(|ret_val| return_error::<R>(method_name, &ret_val))
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

impl_varcall_signature_for_tuple!(0, R);
impl_ptrcall_signature_for_tuple!(R);
impl_varcall_signature_for_tuple!(1, R, P0: 0);
impl_ptrcall_signature_for_tuple!(R, P0: 0);
impl_varcall_signature_for_tuple!(2, R, P0: 0, P1: 1);
impl_ptrcall_signature_for_tuple!(R, P0: 0, P1: 1);
impl_varcall_signature_for_tuple!(3, R, P0: 0, P1: 1, P2: 2);
impl_ptrcall_signature_for_tuple!(R, P0: 0, P1: 1, P2: 2);
impl_varcall_signature_for_tuple!(4, R, P0: 0, P1: 1, P2: 2, P3: 3);
impl_ptrcall_signature_for_tuple!(R, P0: 0, P1: 1, P2: 2, P3: 3);
impl_varcall_signature_for_tuple!(5, R, P0: 0, P1: 1, P2: 2, P3: 3, P4: 4);
impl_ptrcall_signature_for_tuple!(R, P0: 0, P1: 1, P2: 2, P3: 3, P4: 4);
impl_varcall_signature_for_tuple!(6, R, P0: 0, P1: 1, P2: 2, P3: 3, P4: 4, P5: 5);
impl_ptrcall_signature_for_tuple!(R, P0: 0, P1: 1, P2: 2, P3: 3, P4: 4, P5: 5);
impl_varcall_signature_for_tuple!(7, R, P0: 0, P1: 1, P2: 2, P3: 3, P4: 4, P5: 5, P6: 6);
impl_ptrcall_signature_for_tuple!(R, P0: 0, P1: 1, P2: 2, P3: 3, P4: 4, P5: 5, P6: 6);
impl_varcall_signature_for_tuple!(8, R, P0: 0, P1: 1, P2: 2, P3: 3, P4: 4, P5: 5, P6: 6, P7: 7);
impl_ptrcall_signature_for_tuple!(R, P0: 0, P1: 1, P2: 2, P3: 3, P4: 4, P5: 5, P6: 6, P7: 7);
impl_varcall_signature_for_tuple!(9, R, P0: 0, P1: 1, P2: 2, P3: 3, P4: 4, P5: 5, P6: 6, P7: 7, P8: 8);
impl_ptrcall_signature_for_tuple!(R, P0: 0, P1: 1, P2: 2, P3: 3, P4: 4, P5: 5, P6: 6, P7: 7, P8: 8);
impl_varcall_signature_for_tuple!(10, R, P0: 0, P1: 1, P2: 2, P3: 3, P4: 4, P5: 5, P6: 6, P7: 7, P8: 8, P9: 9);
impl_ptrcall_signature_for_tuple!(R, P0: 0, P1: 1, P2: 2, P3: 3, P4: 4, P5: 5, P6: 6, P7: 7, P8: 8, P9: 9);
