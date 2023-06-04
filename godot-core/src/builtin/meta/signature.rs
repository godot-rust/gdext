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

    // TODO(uninit) - can we use this for varcall/ptrcall?
    // ret: sys::GDExtensionUninitializedVariantPtr
    // ret: sys::GDExtensionUninitializedTypePtr
    unsafe fn varcall<C: GodotClass>(
        instance_ptr: sys::GDExtensionClassInstancePtr,
        args_ptr: *const sys::GDExtensionConstVariantPtr,
        ret: sys::GDExtensionVariantPtr,
        err: *mut sys::GDExtensionCallError,
        func: fn(sys::GDExtensionClassInstancePtr, Self::Params) -> Self::Ret,
        method_name: &str,
    );

    // Note: this method imposes extra bounds on GodotFfi, which may not be implemented for user types.
    // We could fall back to varcalls in such cases, and not require GodotFfi categorically.
    unsafe fn ptrcall<C: GodotClass>(
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
                func: fn(sys::GDExtensionClassInstancePtr, Self::Params) -> Self::Ret,
                method_name: &str,
            ) {
                $crate::out!("varcall: {}", method_name);

                let args = ($(
                    unsafe { varcall_arg::<$Pn, $n>(args_ptr, method_name) },
                )*) ;

                varcall_return::<$R>(func(instance_ptr, args), ret, err)
            }

            #[inline]
            unsafe fn ptrcall<C : GodotClass>(
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
