/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::fmt::Debug;

use godot_ffi as sys;
use sys::{BuiltinMethodBind, ClassMethodBind, UtilityFunctionBind};

// TODO:
// separate arguments and return values, so that a type can be used in function arguments even if it doesn't
// implement `ToGodot`, and the other way around for return values.

use crate::builtin::meta::*;
use crate::builtin::Variant;
use crate::obj::InstanceId;

#[doc(hidden)]
pub trait VarcallSignatureTuple: PtrcallSignatureTuple {
    const PARAM_COUNT: usize;

    fn param_property_info(index: usize, param_name: &str) -> PropertyInfo;
    fn param_info(index: usize, param_name: &str) -> Option<MethodParamOrReturnInfo>;
    fn return_info() -> Option<MethodParamOrReturnInfo>;

    // TODO(uninit) - can we use this for varcall/ptrcall?
    // ret: sys::GDExtensionUninitializedVariantPtr
    // ret: sys::GDExtensionUninitializedTypePtr
    unsafe fn in_varcall(
        instance_ptr: sys::GDExtensionClassInstancePtr,
        method_name: &str,
        args_ptr: *const sys::GDExtensionConstVariantPtr,
        ret: sys::GDExtensionVariantPtr,
        err: *mut sys::GDExtensionCallError,
        func: fn(sys::GDExtensionClassInstancePtr, Self::Params) -> Self::Ret,
    );

    unsafe fn out_class_varcall(
        method_bind: sys::GDExtensionMethodBindPtr,
        method_name: &'static str,
        object_ptr: sys::GDExtensionObjectPtr,
        maybe_instance_id: Option<InstanceId>, // if not static
        args: Self::Params,
        varargs: &[Variant],
    ) -> Self::Ret;

    unsafe fn out_utility_ptrcall_varargs(
        utility_fn: UtilityFunctionBind,
        method_name: &'static str,
        args: Self::Params,
        varargs: &[Variant],
    ) -> Self::Ret;

    fn format_args(args: &Self::Params) -> String;
}

#[doc(hidden)]
pub trait PtrcallSignatureTuple {
    type Params;
    type Ret;

    // Note: this method imposes extra bounds on GodotFfi, which may not be implemented for user types.
    // We could fall back to varcalls in such cases, and not require GodotFfi categorically.
    unsafe fn in_ptrcall(
        instance_ptr: sys::GDExtensionClassInstancePtr,
        method_name: &'static str,
        args_ptr: *const sys::GDExtensionConstTypePtr,
        ret: sys::GDExtensionTypePtr,
        func: fn(sys::GDExtensionClassInstancePtr, Self::Params) -> Self::Ret,
        call_type: sys::PtrcallType,
    );

    unsafe fn out_class_ptrcall<Rr: PtrcallReturn<Ret = Self::Ret>>(
        method_bind: sys::GDExtensionMethodBindPtr,
        method_name: &'static str,
        object_ptr: sys::GDExtensionObjectPtr,
        maybe_instance_id: Option<InstanceId>, // if not static
        args: Self::Params,
    ) -> Self::Ret;

    unsafe fn out_builtin_ptrcall<Rr: PtrcallReturn<Ret = Self::Ret>>(
        builtin_fn: BuiltinMethodBind,
        method_name: &'static str,
        type_ptr: sys::GDExtensionTypePtr,
        args: Self::Params,
    ) -> Self::Ret;

    unsafe fn out_utility_ptrcall(
        utility_fn: UtilityFunctionBind,
        method_name: &'static str,
        args: Self::Params,
    ) -> Self::Ret;
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

macro_rules! impl_varcall_signature_for_tuple {
    (
        $PARAM_COUNT:literal;
        $R:ident
        $(, ($pn:ident, $n:tt) : $Pn:ident)* // $n cannot be literal if substituted as tuple index .0
    ) => {
        #[allow(unused_variables)]
        impl<$R, $($Pn,)*> VarcallSignatureTuple for ($R, $($Pn,)*)
            where
                $R: ToGodot + FromGodot + FromVariantIndirect + Debug,
                $(
                    $Pn: ToGodot + FromGodot + Debug,
                )*
        {
            const PARAM_COUNT: usize = $PARAM_COUNT;

            #[inline]
            fn param_info(index: usize, param_name: &str) -> Option<MethodParamOrReturnInfo> {
                match index {
                    $(
                        $n => Some($Pn::Via::argument_info(param_name)),
                    )*
                    _ => None,
                }
            }

            #[inline]
            fn return_info() -> Option<MethodParamOrReturnInfo> {
                $R::Via::return_info()
            }

            #[inline]
            fn param_property_info(index: usize, param_name: &str) -> PropertyInfo {
                match index {
                    $(
                        $n => $Pn::Via::property_info(param_name),
                    )*
                    _ => unreachable!("property_info: unavailable for index {}", index),
                }
            }

            #[inline]
            unsafe fn in_varcall(
                instance_ptr: sys::GDExtensionClassInstancePtr,
                method_name: &str,
                args_ptr: *const sys::GDExtensionConstVariantPtr,
                ret: sys::GDExtensionVariantPtr,
                err: *mut sys::GDExtensionCallError,
                func: fn(sys::GDExtensionClassInstancePtr, Self::Params) -> Self::Ret,
            ) {
                //$crate::out!("in_varcall: {method_name}");
                let args = ($(
                    unsafe { varcall_arg::<$Pn, $n>(args_ptr, method_name) },
                )*) ;

                varcall_return::<$R>(func(instance_ptr, args), ret, err)
            }

            #[inline]
            unsafe fn out_class_varcall(
                method_bind: ClassMethodBind,
                method_name: &'static str,
                object_ptr: sys::GDExtensionObjectPtr,
                maybe_instance_id: Option<InstanceId>, // if not static
                ($($pn,)*): Self::Params,
                varargs: &[Variant],
            ) -> Self::Ret {
                //$crate::out!("out_class_varcall: {method_name}");

                // Note: varcalls are not safe from failing, if the happen through an object pointer -> validity check necessary.
                if let Some(instance_id) = maybe_instance_id {
                    crate::engine::ensure_object_alive(Some(instance_id), object_ptr, method_name);
                }

                let class_fn = sys::interface_fn!(object_method_bind_call);

                let explicit_args = [
                    $(
                        GodotFfiVariant::ffi_to_variant(&into_ffi($pn)),
                    )*
                ];

                let mut variant_ptrs = Vec::with_capacity(explicit_args.len() + varargs.len());
                variant_ptrs.extend(explicit_args.iter().map(Variant::var_sys_const));
                variant_ptrs.extend(varargs.iter().map(Variant::var_sys_const));

                let variant = Variant::from_var_sys_init(|return_ptr| {
                    let mut err = sys::default_call_error();
                    class_fn(
                        method_bind,
                        object_ptr,
                        variant_ptrs.as_ptr(),
                        variant_ptrs.len() as i64,
                        return_ptr,
                        std::ptr::addr_of_mut!(err),
                    );

                    check_varcall_error(&err, method_name, &explicit_args, varargs);
                });

                let result = <Self::Ret as FromGodot>::try_from_variant(&variant);
                result.unwrap_or_else(|err| return_error::<Self::Ret>(method_name, err))
            }

            // Note: this is doing a ptrcall, but uses variant conversions for it
            #[inline]
            unsafe fn out_utility_ptrcall_varargs(
                utility_fn: UtilityFunctionBind,
                method_name: &str,
                ($($pn,)*): Self::Params,
                varargs: &[Variant],
            ) -> Self::Ret {
                //$crate::out!("out_utility_ptrcall_varargs: {method_name}");
                let explicit_args: [Variant; $PARAM_COUNT] = [
                    $(
                        GodotFfiVariant::ffi_to_variant(&into_ffi($pn)),
                    )*
                ];

                let mut type_ptrs = Vec::with_capacity(explicit_args.len() + varargs.len());
                type_ptrs.extend(explicit_args.iter().map(sys::GodotFfi::sys_const));
                type_ptrs.extend(varargs.iter().map(sys::GodotFfi::sys_const));

                // Important: this calls from_sys_init_default().
                let result = PtrcallReturnT::<$R>::call(|return_ptr| {
                    utility_fn(return_ptr, type_ptrs.as_ptr(), type_ptrs.len() as i32);
                });
                result.unwrap_or_else(|err| return_error::<Self::Ret>(method_name, err))
            }

            #[inline]
            fn format_args(args: &Self::Params) -> String {
                let mut string = String::new();
                $(
                    string.push_str(&format!("{:?}, ", args.$n));
                )*
                string.remove(string.len() - 2); // remove trailing ", "
                string
            }
        }
    };
}

macro_rules! impl_ptrcall_signature_for_tuple {
    (
        $R:ident
        $(, ($pn:ident, $n:tt) : $Pn:ident)* // $n cannot be literal if substituted as tuple index .0
    ) => {
        #[allow(unused_variables)]
        impl<$R, $($Pn,)*> PtrcallSignatureTuple for ($R, $($Pn,)*)
            where $R: ToGodot + FromGodot + Debug,
               $( $Pn: ToGodot + FromGodot + Debug, )*
        {
            type Params = ($($Pn,)*);
            type Ret = $R;

            #[inline]
            unsafe fn in_ptrcall(
                instance_ptr: sys::GDExtensionClassInstancePtr,
                method_name: &str,
                args_ptr: *const sys::GDExtensionConstTypePtr,
                ret: sys::GDExtensionTypePtr,
                func: fn(sys::GDExtensionClassInstancePtr, Self::Params) -> Self::Ret,
                call_type: sys::PtrcallType,
            ) {
                // $crate::out!("in_ptrcall: {method_name}");
                let args = ($(
                    unsafe { ptrcall_arg::<$Pn, $n>(args_ptr, method_name, call_type) },
                )*) ;

                // SAFETY:
                // `ret` is always a pointer to an initialized value of type $R
                // TODO: double-check the above
                ptrcall_return::<$R>(func(instance_ptr, args), ret, method_name, call_type)
            }

            #[inline]
            unsafe fn out_class_ptrcall<Rr: PtrcallReturn<Ret = Self::Ret>>(
                method_bind: ClassMethodBind,
                method_name: &'static str,
                object_ptr: sys::GDExtensionObjectPtr,
                maybe_instance_id: Option<InstanceId>, // if not static
                ($($pn,)*): Self::Params,
            ) -> Self::Ret {
                // $crate::out!("out_class_ptrcall: {method_name}");
                if let Some(instance_id) = maybe_instance_id {
                    crate::engine::ensure_object_alive(Some(instance_id), object_ptr, method_name);
                }

                let class_fn = sys::interface_fn!(object_method_bind_ptrcall);

                #[allow(clippy::let_unit_value)]
                let marshalled_args = (
                    $(
                        into_ffi($pn),
                    )*
                );

                let type_ptrs = [
                    $(
                        sys::GodotFfi::as_arg_ptr(&marshalled_args.$n),
                    )*
                ];

                let result = Rr::call(|return_ptr| {
                    class_fn(method_bind, object_ptr, type_ptrs.as_ptr(), return_ptr);
                });
                result.unwrap_or_else(|err| return_error::<Self::Ret>(method_name, err))
            }

            #[inline]
            unsafe fn out_builtin_ptrcall<Rr: PtrcallReturn<Ret = Self::Ret>>(
                builtin_fn: BuiltinMethodBind,
                method_name: &'static str,
                type_ptr: sys::GDExtensionTypePtr,
                ($($pn,)*): Self::Params,
            ) -> Self::Ret {
                // $crate::out!("out_builtin_ptrcall: {method_name}");
                #[allow(clippy::let_unit_value)]
                let marshalled_args = (
                    $(
                        into_ffi($pn),
                    )*
                );

                let type_ptrs = [
                    $(
                        sys::GodotFfi::as_arg_ptr(&marshalled_args.$n),
                    )*
                ];

                let result = Rr::call(|return_ptr| {
                    builtin_fn(type_ptr, type_ptrs.as_ptr(), return_ptr, type_ptrs.len() as i32);
                });
                result.unwrap_or_else(|err| return_error::<Self::Ret>(method_name, err))
            }

            #[inline]
            unsafe fn out_utility_ptrcall(
                utility_fn: UtilityFunctionBind,
                method_name: &'static str,
                ($($pn,)*): Self::Params,
            ) -> Self::Ret {
                // $crate::out!("out_utility_ptrcall: {method_name}");
                #[allow(clippy::let_unit_value)]
                let marshalled_args = (
                    $(
                        into_ffi($pn),
                    )*
                );

                let arg_ptrs = [
                    $(
                        sys::GodotFfi::as_arg_ptr(&marshalled_args.$n),
                    )*
                ];

                let result = PtrcallReturnT::<$R>::call(|return_ptr| {
                    utility_fn(return_ptr, arg_ptrs.as_ptr(), arg_ptrs.len() as i32);
                });
                result.unwrap_or_else(|err| return_error::<Self::Ret>(method_name, err))
            }
        }
    };
}

/// Convert the `N`th argument of `args_ptr` into a value of type `P`.
///
/// # Safety
/// - It must be safe to dereference the pointer at `args_ptr.offset(N)` .
unsafe fn varcall_arg<P: FromGodot, const N: isize>(
    args_ptr: *const sys::GDExtensionConstVariantPtr,
    method_name: &str,
) -> P {
    let variant_ref = &*Variant::ptr_from_sys(*args_ptr.offset(N));

    let result = P::try_from_variant(variant_ref);
    result.unwrap_or_else(|err| param_error::<P>(method_name, N as i32, err))
}

/// Moves `ret_val` into `ret`.
///
/// # Safety
/// - `ret` must be a pointer to an initialized `Variant`.
/// - It must be safe to write a `Variant` once to `ret`.
/// - It must be safe to write a `sys::GDExtensionCallError` once to `err`.
unsafe fn varcall_return<R: ToGodot>(
    ret_val: R,
    ret: sys::GDExtensionVariantPtr,
    err: *mut sys::GDExtensionCallError,
) {
    let ret_variant = ret_val.to_variant();
    *(ret as *mut Variant) = ret_variant;
    (*err).error = sys::GDEXTENSION_CALL_OK;
}

/// Moves `ret_val` into `ret`, if it is `Ok(...)`. Otherwise sets an error.
///
/// # Safety
/// See [`varcall_return`].
#[cfg(since_api = "4.2")] // unused before
pub(crate) unsafe fn varcall_return_checked<R: ToGodot>(
    ret_val: Result<R, ()>, // TODO Err should be custom CallError enum
    ret: sys::GDExtensionVariantPtr,
    err: *mut sys::GDExtensionCallError,
) {
    if let Ok(ret_val) = ret_val {
        varcall_return(ret_val, ret, err);
    } else {
        *err = sys::default_call_error();
        (*err).error = sys::GDEXTENSION_CALL_ERROR_INVALID_ARGUMENT;
    }
}

/// Convert the `N`th argument of `args_ptr` into a value of type `P`.
///
/// # Safety
/// - It must be safe to dereference the address at `args_ptr.offset(N)` .
/// - The pointer at `args_ptr.offset(N)` must follow the safety requirements as laid out in
///   [`GodotFuncMarshal::try_from_arg`][sys::GodotFuncMarshal::try_from_arg].
unsafe fn ptrcall_arg<P: FromGodot, const N: isize>(
    args_ptr: *const sys::GDExtensionConstTypePtr,
    method_name: &str,
    call_type: sys::PtrcallType,
) -> P {
    let ffi = <P::Via as GodotType>::Ffi::from_arg_ptr(
        sys::force_mut_ptr(*args_ptr.offset(N)),
        call_type,
    );

    try_from_ffi(ffi).unwrap_or_else(|err| param_error::<P>(method_name, N as i32, err))
}

/// Moves `ret_val` into `ret`.
///
/// # Safety
/// `ret_val`, `ret`, and `call_type` must follow the safety requirements as laid out in
/// [`GodotFuncMarshal::try_return`](sys::GodotFuncMarshal::try_return).
unsafe fn ptrcall_return<R: ToGodot>(
    ret_val: R,
    ret: sys::GDExtensionTypePtr,
    _method_name: &str,
    call_type: sys::PtrcallType,
) {
    let val = into_ffi(ret_val);
    val.move_return_ptr(ret, call_type);
}

fn param_error<P>(method_name: &str, index: i32, err: ConvertError) -> ! {
    let param_ty = std::any::type_name::<P>();
    panic!("in method `{method_name}` at parameter [{index}] of type {param_ty}: {err}",);
}

fn return_error<R>(method_name: &str, err: ConvertError) -> ! {
    let return_ty = std::any::type_name::<R>();
    panic!("in method `{method_name}` at return type {return_ty}: {err}",);
}

fn check_varcall_error<T>(
    err: &sys::GDExtensionCallError,
    fn_name: &str,
    explicit_args: &[T],
    varargs: &[Variant],
) where
    T: Debug + ToGodot,
{
    if err.error == sys::GDEXTENSION_CALL_OK {
        return;
    }

    // TODO(optimize): split into non-generic, expensive parts after error check

    let mut arg_types = Vec::with_capacity(explicit_args.len() + varargs.len());
    arg_types.extend(explicit_args.iter().map(|arg| arg.to_variant().get_type()));
    arg_types.extend(varargs.iter().map(Variant::get_type));

    let explicit_args_str = join_to_string(explicit_args);
    let vararg_str = join_to_string(varargs);

    let func_str = format!("{fn_name}({explicit_args_str}; varargs {vararg_str})");

    sys::panic_call_error(err, &func_str, &arg_types);
}

fn join_to_string<T: Debug>(list: &[T]) -> String {
    list.iter()
        .map(|v| format!("{v:?}"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Helper trait to support `()` which doesn't implement `FromVariant`.
trait FromVariantIndirect {
    fn convert(variant: Variant) -> Self;
}

impl<T: FromGodot> FromVariantIndirect for T {
    fn convert(variant: Variant) -> Self {
        T::from_variant(&variant)
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Poor man's variadic templates.
// For example, RenderingServer::environment_set_volumetric_fog() has 14 parameters. We may need to extend this if the API adds more such methods.

impl_varcall_signature_for_tuple!(0; R);
impl_varcall_signature_for_tuple!(1; R, (p0, 0): P0);
impl_varcall_signature_for_tuple!(2; R, (p0, 0): P0, (p1, 1): P1);
impl_varcall_signature_for_tuple!(3; R, (p0, 0): P0, (p1, 1): P1, (p2, 2): P2);
impl_varcall_signature_for_tuple!(4; R, (p0, 0): P0, (p1, 1): P1, (p2, 2): P2, (p3, 3): P3);
impl_varcall_signature_for_tuple!(5; R, (p0, 0): P0, (p1, 1): P1, (p2, 2): P2, (p3, 3): P3, (p4, 4): P4);
impl_varcall_signature_for_tuple!(6; R, (p0, 0): P0, (p1, 1): P1, (p2, 2): P2, (p3, 3): P3, (p4, 4): P4, (p5, 5): P5);
impl_varcall_signature_for_tuple!(7; R, (p0, 0): P0, (p1, 1): P1, (p2, 2): P2, (p3, 3): P3, (p4, 4): P4, (p5, 5): P5, (p6, 6): P6);
impl_varcall_signature_for_tuple!(8; R, (p0, 0): P0, (p1, 1): P1, (p2, 2): P2, (p3, 3): P3, (p4, 4): P4, (p5, 5): P5, (p6, 6): P6, (p7, 7): P7);
impl_varcall_signature_for_tuple!(9; R, (p0, 0): P0, (p1, 1): P1, (p2, 2): P2, (p3, 3): P3, (p4, 4): P4, (p5, 5): P5, (p6, 6): P6, (p7, 7): P7, (p8, 8): P8);
impl_varcall_signature_for_tuple!(10; R, (p0, 0): P0, (p1, 1): P1, (p2, 2): P2, (p3, 3): P3, (p4, 4): P4, (p5, 5): P5, (p6, 6): P6, (p7, 7): P7, (p8, 8): P8, (p9, 9): P9);
impl_varcall_signature_for_tuple!(11; R, (p0, 0): P0, (p1, 1): P1, (p2, 2): P2, (p3, 3): P3, (p4, 4): P4, (p5, 5): P5, (p6, 6): P6, (p7, 7): P7, (p8, 8): P8, (p9, 9): P9, (p10, 10): P10);
impl_varcall_signature_for_tuple!(12; R, (p0, 0): P0, (p1, 1): P1, (p2, 2): P2, (p3, 3): P3, (p4, 4): P4, (p5, 5): P5, (p6, 6): P6, (p7, 7): P7, (p8, 8): P8, (p9, 9): P9, (p10, 10): P10, (p11, 11): P11);
impl_varcall_signature_for_tuple!(13; R, (p0, 0): P0, (p1, 1): P1, (p2, 2): P2, (p3, 3): P3, (p4, 4): P4, (p5, 5): P5, (p6, 6): P6, (p7, 7): P7, (p8, 8): P8, (p9, 9): P9, (p10, 10): P10, (p11, 11): P11, (p12, 12): P12);
impl_varcall_signature_for_tuple!(14; R, (p0, 0): P0, (p1, 1): P1, (p2, 2): P2, (p3, 3): P3, (p4, 4): P4, (p5, 5): P5, (p6, 6): P6, (p7, 7): P7, (p8, 8): P8, (p9, 9): P9, (p10, 10): P10, (p11, 11): P11, (p12, 12): P12, (p13, 13): P13);

impl_ptrcall_signature_for_tuple!(R);
impl_ptrcall_signature_for_tuple!(R, (p0, 0): P0);
impl_ptrcall_signature_for_tuple!(R, (p0, 0): P0, (p1, 1): P1);
impl_ptrcall_signature_for_tuple!(R, (p0, 0): P0, (p1, 1): P1, (p2, 2): P2);
impl_ptrcall_signature_for_tuple!(R, (p0, 0): P0, (p1, 1): P1, (p2, 2): P2, (p3, 3): P3);
impl_ptrcall_signature_for_tuple!(R, (p0, 0): P0, (p1, 1): P1, (p2, 2): P2, (p3, 3): P3, (p4, 4): P4);
impl_ptrcall_signature_for_tuple!(R, (p0, 0): P0, (p1, 1): P1, (p2, 2): P2, (p3, 3): P3, (p4, 4): P4, (p5, 5): P5);
impl_ptrcall_signature_for_tuple!(R, (p0, 0): P0, (p1, 1): P1, (p2, 2): P2, (p3, 3): P3, (p4, 4): P4, (p5, 5): P5, (p6, 6): P6);
impl_ptrcall_signature_for_tuple!(R, (p0, 0): P0, (p1, 1): P1, (p2, 2): P2, (p3, 3): P3, (p4, 4): P4, (p5, 5): P5, (p6, 6): P6, (p7, 7): P7);
impl_ptrcall_signature_for_tuple!(R, (p0, 0): P0, (p1, 1): P1, (p2, 2): P2, (p3, 3): P3, (p4, 4): P4, (p5, 5): P5, (p6, 6): P6, (p7, 7): P7, (p8, 8): P8);
impl_ptrcall_signature_for_tuple!(R, (p0, 0): P0, (p1, 1): P1, (p2, 2): P2, (p3, 3): P3, (p4, 4): P4, (p5, 5): P5, (p6, 6): P6, (p7, 7): P7, (p8, 8): P8, (p9, 9): P9);
impl_ptrcall_signature_for_tuple!(R, (p0, 0): P0, (p1, 1): P1, (p2, 2): P2, (p3, 3): P3, (p4, 4): P4, (p5, 5): P5, (p6, 6): P6, (p7, 7): P7, (p8, 8): P8, (p9, 9): P9, (p10, 10): P10);
impl_ptrcall_signature_for_tuple!(R, (p0, 0): P0, (p1, 1): P1, (p2, 2): P2, (p3, 3): P3, (p4, 4): P4, (p5, 5): P5, (p6, 6): P6, (p7, 7): P7, (p8, 8): P8, (p9, 9): P9, (p10, 10): P10, (p11, 11): P11);
impl_ptrcall_signature_for_tuple!(R, (p0, 0): P0, (p1, 1): P1, (p2, 2): P2, (p3, 3): P3, (p4, 4): P4, (p5, 5): P5, (p6, 6): P6, (p7, 7): P7, (p8, 8): P8, (p9, 9): P9, (p10, 10): P10, (p11, 11): P11, (p12, 12): P12);
impl_ptrcall_signature_for_tuple!(R, (p0, 0): P0, (p1, 1): P1, (p2, 2): P2, (p3, 3): P3, (p4, 4): P4, (p5, 5): P5, (p6, 6): P6, (p7, 7): P7, (p8, 8): P8, (p9, 9): P9, (p10, 10): P10, (p11, 11): P11, (p12, 12): P12, (p13, 13): P13);
