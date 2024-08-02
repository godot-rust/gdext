/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
use std::borrow::Cow;
use std::fmt;
use std::fmt::Debug;

use godot_ffi as sys;
use sys::{BuiltinMethodBind, ClassMethodBind, GodotFfi, UtilityFunctionBind};

use crate::builtin::Variant;
use crate::meta::error::{CallError, ConvertError};
use crate::meta::godot_convert::{into_ffi, try_from_ffi};
use crate::meta::*;
use crate::obj::{GodotClass, InstanceId};

// TODO:
// separate arguments and return values, so that a type can be used in function arguments even if it doesn't
// implement `ToGodot`, and the other way around for return values.

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
        call_ctx: &CallContext,
        args_ptr: *const sys::GDExtensionConstVariantPtr,
        arg_count: i64,
        ret: sys::GDExtensionVariantPtr,
        err: *mut sys::GDExtensionCallError,
        func: fn(sys::GDExtensionClassInstancePtr, Self::Params) -> Self::Ret,
    ) -> Result<(), CallError>;

    unsafe fn out_class_varcall(
        method_bind: ClassMethodBind,
        // Separate parameters to reduce tokens in generated class API.
        class_name: &'static str,
        method_name: &'static str,
        object_ptr: sys::GDExtensionObjectPtr,
        maybe_instance_id: Option<InstanceId>, // if not static
        args: Self::Params,
        varargs: &[Variant],
    ) -> Result<Self::Ret, CallError>;

    /// Outbound virtual call to a method overridden by a script attached to the object.
    ///
    /// Returns `None` if the script does not override the method.
    #[cfg(since_api = "4.3")]
    unsafe fn out_script_virtual_call(
        // Separate parameters to reduce tokens in macro-generated API.
        class_name: &'static str,
        method_name: &'static str,
        method_sname_ptr: sys::GDExtensionConstStringNamePtr,
        object_ptr: sys::GDExtensionObjectPtr,
        args: Self::Params,
    ) -> Self::Ret;

    unsafe fn out_utility_ptrcall_varargs(
        utility_fn: UtilityFunctionBind,
        function_name: &'static str,
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
        call_ctx: &CallContext<'static>,
        args_ptr: *const sys::GDExtensionConstTypePtr,
        ret: sys::GDExtensionTypePtr,
        func: fn(sys::GDExtensionClassInstancePtr, Self::Params) -> Self::Ret,
        call_type: sys::PtrcallType,
    );

    unsafe fn out_class_ptrcall(
        method_bind: ClassMethodBind,
        // Separate parameters to reduce tokens in generated class API.
        class_name: &'static str,
        method_name: &'static str,
        object_ptr: sys::GDExtensionObjectPtr,
        maybe_instance_id: Option<InstanceId>, // if not static
        args: Self::Params,
    ) -> Self::Ret;

    unsafe fn out_builtin_ptrcall(
        builtin_fn: BuiltinMethodBind,
        // Separate parameters to reduce tokens in generated class API.
        class_name: &'static str,
        method_name: &'static str,
        type_ptr: sys::GDExtensionTypePtr,
        args: Self::Params,
    ) -> Self::Ret;

    unsafe fn out_utility_ptrcall(
        utility_fn: UtilityFunctionBind,
        function_name: &'static str,
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
                $R: ToGodot + FromGodot + Debug,
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
                call_ctx: &CallContext,
                args_ptr: *const sys::GDExtensionConstVariantPtr,
                arg_count: i64,
                ret: sys::GDExtensionVariantPtr,
                err: *mut sys::GDExtensionCallError,
                func: fn(sys::GDExtensionClassInstancePtr, Self::Params) -> Self::Ret,
            ) -> Result<(), CallError> {
                //$crate::out!("in_varcall: {call_ctx}");
                CallError::check_arg_count(call_ctx, arg_count as usize, $PARAM_COUNT)?;

                #[cfg(feature = "trace")]
                trace::push(true, false, &call_ctx);

                let args = ($(
                    unsafe { varcall_arg::<$Pn, $n>(args_ptr, call_ctx)? },
                )*) ;

                let rust_result = func(instance_ptr, args);
                varcall_return::<$R>(rust_result, ret, err);
                Ok(())
            }

            #[inline]
            unsafe fn out_class_varcall(
                method_bind: ClassMethodBind,
                // Separate parameters to reduce tokens in generated class API.
                class_name: &'static str,
                method_name: &'static str,
                object_ptr: sys::GDExtensionObjectPtr,
                maybe_instance_id: Option<InstanceId>, // if not static
                ($($pn,)*): Self::Params,
                varargs: &[Variant],
            ) -> Result<Self::Ret, CallError> {
                let call_ctx = CallContext::outbound(class_name, method_name);
                //$crate::out!("out_class_varcall: {call_ctx}");

                // Note: varcalls are not safe from failing, if they happen through an object pointer -> validity check necessary.
                if let Some(instance_id) = maybe_instance_id {
                    crate::classes::ensure_object_alive(instance_id, object_ptr, &call_ctx);
                }

                let class_fn = sys::interface_fn!(object_method_bind_call);

                let explicit_args = [
                    $(
                        GodotFfiVariant::ffi_to_variant(&into_ffi($pn)),
                    )*
                ];

                let mut variant_ptrs = Vec::with_capacity(explicit_args.len() + varargs.len());
                variant_ptrs.extend(explicit_args.iter().map(Variant::var_sys));
                variant_ptrs.extend(varargs.iter().map(Variant::var_sys));

                let variant: Result<Variant, CallError> = Variant::new_with_var_uninit_result(|return_ptr| {
                    let mut err = sys::default_call_error();
                    class_fn(
                        method_bind.0,
                        object_ptr,
                        variant_ptrs.as_ptr(),
                        variant_ptrs.len() as i64,
                        return_ptr,
                        std::ptr::addr_of_mut!(err),
                    );

                    CallError::check_out_varcall(&call_ctx, err, &explicit_args, varargs)
                });

                variant.and_then(|v| {
                    v.try_to::<Self::Ret>()
                        .map_err(|e| CallError::failed_return_conversion::<Self::Ret>(&call_ctx, e))
                })
            }

            #[cfg(since_api = "4.3")]
            unsafe fn out_script_virtual_call(
                // Separate parameters to reduce tokens in macro-generated API.
                class_name: &'static str,
                method_name: &'static str,
                method_sname_ptr: sys::GDExtensionConstStringNamePtr,
                object_ptr: sys::GDExtensionObjectPtr,
                ($($pn,)*): Self::Params,
            ) -> Self::Ret {
                // Assumes that caller has previously checked existence of a virtual method.

                let call_ctx = CallContext::outbound(class_name, method_name);
                //$crate::out!("out_script_virtual_call: {call_ctx}");

                let object_call_script_method = sys::interface_fn!(object_call_script_method);
                let explicit_args = [
                    $(
                        GodotFfiVariant::ffi_to_variant(&into_ffi($pn)),
                    )*
                ];

                let variant_ptrs = explicit_args.iter().map(Variant::var_sys).collect::<Vec<_>>();

                let variant = Variant::new_with_var_uninit(|return_ptr| {
                    let mut err = sys::default_call_error();
                    object_call_script_method(
                        object_ptr,
                        method_sname_ptr,
                        variant_ptrs.as_ptr(),
                        variant_ptrs.len() as i64,
                        return_ptr,
                        std::ptr::addr_of_mut!(err),
                    );
                });

                let result = <Self::Ret as FromGodot>::try_from_variant(&variant);
                result.unwrap_or_else(|err| return_error::<Self::Ret>(&call_ctx, err))
            }

            // Note: this is doing a ptrcall, but uses variant conversions for it.
            #[inline]
            unsafe fn out_utility_ptrcall_varargs(
                utility_fn: UtilityFunctionBind,
                function_name: &'static str,
                ($($pn,)*): Self::Params,
                varargs: &[Variant],
            ) -> Self::Ret {
                let call_ctx = CallContext::outbound("", function_name);
                //$crate::out!("out_utility_ptrcall_varargs: {call_ctx}");

                let explicit_args: [Variant; $PARAM_COUNT] = [
                    $(
                        GodotFfiVariant::ffi_to_variant(&into_ffi($pn)),
                    )*
                ];

                let mut type_ptrs = Vec::with_capacity(explicit_args.len() + varargs.len());
                type_ptrs.extend(explicit_args.iter().map(sys::GodotFfi::sys));
                type_ptrs.extend(varargs.iter().map(sys::GodotFfi::sys));

                // Important: this calls from_sys_init_default().
                let result = new_from_ptrcall::<Self::Ret>(|return_ptr| {
                    utility_fn(return_ptr, type_ptrs.as_ptr(), type_ptrs.len() as i32);
                });
                result.unwrap_or_else(|err| return_error::<Self::Ret>(&call_ctx, err))
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
                call_ctx: &CallContext,
                args_ptr: *const sys::GDExtensionConstTypePtr,
                ret: sys::GDExtensionTypePtr,
                func: fn(sys::GDExtensionClassInstancePtr, Self::Params) -> Self::Ret,
                call_type: sys::PtrcallType,
            ) {
                // $crate::out!("in_ptrcall: {call_ctx}");

                #[cfg(feature = "trace")]
                trace::push(true, true, &call_ctx);

                let args = ($(
                    unsafe { ptrcall_arg::<$Pn, $n>(args_ptr, call_ctx, call_type) },
                )*) ;

                // SAFETY:
                // `ret` is always a pointer to an initialized value of type $R
                // TODO: double-check the above
                ptrcall_return::<$R>(func(instance_ptr, args), ret, call_ctx, call_type)
            }

            #[inline]
            unsafe fn out_class_ptrcall(
                method_bind: ClassMethodBind,
                // Separate parameters to reduce tokens in generated class API.
                class_name: &'static str,
                method_name: &'static str,
                object_ptr: sys::GDExtensionObjectPtr,
                maybe_instance_id: Option<InstanceId>, // if not static
                ($($pn,)*): Self::Params,
            ) -> Self::Ret {
                let call_ctx = CallContext::outbound(class_name, method_name);
                // $crate::out!("out_class_ptrcall: {call_ctx}");

                if let Some(instance_id) = maybe_instance_id {
                    crate::classes::ensure_object_alive(instance_id, object_ptr, &call_ctx);
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

                let result = new_from_ptrcall::<Self::Ret>(|return_ptr| {
                    class_fn(method_bind.0, object_ptr, type_ptrs.as_ptr(), return_ptr);
                });
                result.unwrap_or_else(|err| return_error::<Self::Ret>(&call_ctx, err))
            }

            #[inline]
            unsafe fn out_builtin_ptrcall(
                builtin_fn: BuiltinMethodBind,
                // Separate parameters to reduce tokens in generated class API.
                class_name: &'static str,
                method_name: &'static str,
                type_ptr: sys::GDExtensionTypePtr,
                ($($pn,)*): Self::Params,
            ) -> Self::Ret {
                let call_ctx = CallContext::outbound(class_name, method_name);
                // $crate::out!("out_builtin_ptrcall: {call_ctx}");

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

                let result = new_from_ptrcall::<Self::Ret>(|return_ptr| {
                    builtin_fn(type_ptr, type_ptrs.as_ptr(), return_ptr, type_ptrs.len() as i32);
                });
                result.unwrap_or_else(|err| return_error::<Self::Ret>(&call_ctx, err))
            }

            #[inline]
            unsafe fn out_utility_ptrcall(
                utility_fn: UtilityFunctionBind,
                function_name: &'static str,
                ($($pn,)*): Self::Params,
            ) -> Self::Ret {
                let call_ctx = CallContext::outbound("", function_name);
                // $crate::out!("out_utility_ptrcall: {call_ctx}");

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

                let result = new_from_ptrcall::<Self::Ret>(|return_ptr| {
                    utility_fn(return_ptr, arg_ptrs.as_ptr(), arg_ptrs.len() as i32);
                });
                result.unwrap_or_else(|err| return_error::<Self::Ret>(&call_ctx, err))
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
    call_ctx: &CallContext,
) -> Result<P, CallError> {
    let variant_ref = Variant::borrow_var_sys(*args_ptr.offset(N));

    P::try_from_variant(variant_ref)
        .map_err(|err| CallError::failed_param_conversion::<P>(call_ctx, N, err))
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
    call_ctx: &CallContext,
    call_type: sys::PtrcallType,
) -> P {
    let ffi = <P::Via as GodotType>::Ffi::from_arg_ptr(
        sys::force_mut_ptr(*args_ptr.offset(N)),
        call_type,
    );

    try_from_ffi(ffi).unwrap_or_else(|err| param_error::<P>(call_ctx, N as i32, err))
}

/// Moves `ret_val` into `ret`.
///
/// # Safety
/// `ret_val`, `ret`, and `call_type` must follow the safety requirements as laid out in
/// [`GodotFuncMarshal::try_return`](sys::GodotFuncMarshal::try_return).
unsafe fn ptrcall_return<R: ToGodot>(
    ret_val: R,
    ret: sys::GDExtensionTypePtr,
    _call_ctx: &CallContext,
    call_type: sys::PtrcallType,
) {
    let val = into_ffi(ret_val);
    val.move_return_ptr(ret, call_type);
}

fn param_error<P>(call_ctx: &CallContext, index: i32, err: ConvertError) -> ! {
    let param_ty = std::any::type_name::<P>();
    panic!("in function `{call_ctx}` at parameter [{index}] of type {param_ty}: {err}");
}

fn return_error<R>(call_ctx: &CallContext, err: ConvertError) -> ! {
    let return_ty = std::any::type_name::<R>();
    panic!("in function `{call_ctx}` at return type {return_ty}: {err}");
}

unsafe fn new_from_ptrcall<T: FromGodot>(
    process_return_ptr: impl FnOnce(sys::GDExtensionTypePtr),
) -> Result<T, ConvertError> {
    let ffi = <<T::Via as GodotType>::Ffi as sys::GodotFfi>::new_with_init(|return_ptr| {
        process_return_ptr(return_ptr)
    });

    T::Via::try_from_ffi(ffi).and_then(T::try_from_godot)
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

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Information about function and method calls.

// Lazy Display, so we don't create tens of thousands of extra string literals.
#[derive(Clone)]
#[doc(hidden)] // currently exposed in godot::meta
pub struct CallContext<'a> {
    pub(crate) class_name: Cow<'a, str>,
    pub(crate) function_name: &'a str,
}

impl<'a> CallContext<'a> {
    /// Call from Godot into a user-defined #[func] function.
    pub const fn func(class_name: &'a str, function_name: &'a str) -> Self {
        Self {
            class_name: Cow::Borrowed(class_name),
            function_name,
        }
    }

    /// Outbound call from Rust into the engine, class/builtin APIs.
    pub const fn outbound(class_name: &'a str, function_name: &'a str) -> Self {
        Self {
            class_name: Cow::Borrowed(class_name),
            function_name,
        }
    }

    /// Outbound call from Rust into the engine, via Gd methods.
    pub fn gd<T: GodotClass>(function_name: &'a str) -> Self {
        Self {
            class_name: T::class_name().to_cow_str(),
            function_name,
        }
    }
}

impl<'a> fmt::Display for CallContext<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}::{}", self.class_name, self.function_name)
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Trace diagnostics for integration tests
#[cfg(feature = "trace")]
pub mod trace {
    use crate::meta::CallContext;

    use super::sys::Global;

    /// Stores information about the current call for diagnostic purposes.
    pub struct CallReport {
        pub class: String,
        pub method: String,
        pub is_inbound: bool,
        pub is_ptrcall: bool,
    }

    pub fn pop() -> CallReport {
        let lock = TRACE.lock().take();
        // let th = std::thread::current().id();
        // println!("trace::pop [{th:?}]...");

        lock.expect("trace::pop() had no prior call stored.")
    }

    pub(crate) fn push(inbound: bool, ptrcall: bool, call_ctx: &CallContext) {
        if call_ctx.function_name.contains("notrace") {
            return;
        }
        // let th = std::thread::current().id();
        // println!("trace::push [{th:?}] - inbound: {inbound}, ptrcall: {ptrcall}, ctx: {call_ctx}");

        let report = CallReport {
            class: call_ctx.class_name.to_string(),
            method: call_ctx.function_name.to_string(),
            is_inbound: inbound,
            is_ptrcall: ptrcall,
        };

        *TRACE.lock() = Some(report);
    }

    static TRACE: Global<Option<CallReport>> = Global::default();
}
