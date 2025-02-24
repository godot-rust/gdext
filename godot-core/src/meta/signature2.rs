/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::{borrow::Cow, fmt, marker::PhantomData};

use crate::{
    builtin::Variant,
    obj::{GodotClass, InstanceId},
};

use super::{
    error::{CallError, ConvertError},
    godot_convert::try_from_ffi,
    GodotConvert, GodotType, PropertyInfo, ToGodot,
};

use godot_ffi::{self as sys, GodotFfi};

#[cfg(feature = "trace")]
pub use crate::meta::trace;

type CallResult<R> = Result<R, CallError>;
use super::FromGodot;

mod impls;

pub trait ParamList: Sized {
    const LEN: usize;
    fn property_info(index: usize, param_name: &str) -> PropertyInfo;

    fn param_info(
        index: usize,
        param_name: &str,
    ) -> Option<crate::registry::method::MethodParamOrReturnInfo>;

    fn format_args(&self) -> String;
}

pub trait InParamList: ParamList {
    unsafe fn from_varcall_args(
        args_ptr: *const sys::GDExtensionConstVariantPtr,
        call_ctx: &CallContext,
    ) -> CallResult<Self>;

    unsafe fn from_ptrcall_args(
        args_ptr: *const sys::GDExtensionConstTypePtr,
        call_type: sys::PtrcallType,
        call_ctx: &CallContext,
    ) -> Self;
}

pub trait OutParamList: ParamList {
    fn with_args<F, R>(self, f: F) -> R
    where
        F: FnOnce(&[Variant], &[sys::GDExtensionConstVariantPtr]) -> R;

    fn with_ptr_args<F, R>(self, f: F) -> R
    where
        F: FnOnce(&[sys::GDExtensionConstTypePtr]) -> R;
}

pub struct Signature<Params, Ret> {
    _p: PhantomData<Params>,
    _r: PhantomData<Ret>,
}

impl<Params, Ret: GodotConvert> Signature<Params, Ret> {
    fn return_info() -> Option<crate::registry::method::MethodParamOrReturnInfo> {
        Ret::Via::return_info()
    }
}

/// In-calls:
///
/// Calls going from the Godot engine to rust code.
impl<Params: InParamList, Ret: ToGodot> Signature<Params, Ret> {
    #[inline]
    pub unsafe fn in_varcall(
        instance_ptr: sys::GDExtensionClassInstancePtr,
        call_ctx: &CallContext,
        args_ptr: *const sys::GDExtensionConstVariantPtr,
        arg_count: i64,
        ret: sys::GDExtensionVariantPtr,
        err: *mut sys::GDExtensionCallError,
        func: fn(sys::GDExtensionClassInstancePtr, Params) -> Ret,
    ) -> CallResult<()> {
        //$crate::out!("in_varcall: {call_ctx}");
        CallError::check_arg_count(call_ctx, arg_count as usize, Params::LEN)?;

        #[cfg(feature = "trace")]
        trace::push(true, false, &call_ctx);

        let args = Params::from_varcall_args(args_ptr, call_ctx)?;

        let rust_result = func(instance_ptr, args);
        varcall_return::<Ret>(rust_result, ret, err);
        Ok(())
    }

    #[inline]
    pub unsafe fn in_ptrcall(
        instance_ptr: sys::GDExtensionClassInstancePtr,
        call_ctx: &CallContext,
        args_ptr: *const sys::GDExtensionConstTypePtr,
        ret: sys::GDExtensionTypePtr,
        func: fn(sys::GDExtensionClassInstancePtr, Params) -> Ret,
        call_type: sys::PtrcallType,
    ) {
        // $crate::out!("in_ptrcall: {call_ctx}");

        #[cfg(feature = "trace")]
        trace::push(true, true, &call_ctx);

        let args = Params::from_ptrcall_args(args_ptr, call_type, call_ctx);

        // SAFETY:
        // `ret` is always a pointer to an initialized value of type $R
        // TODO: double-check the above
        ptrcall_return::<Ret>(func(instance_ptr, args), ret, call_ctx, call_type)
    }
}

/// Out-calls:
///
/// Calls going from the rust code to the Godot engine.
impl<Params: OutParamList, Ret: FromGodot> Signature<Params, Ret> {
    #[inline]
    pub unsafe fn out_class_varcall(
        method_bind: sys::ClassMethodBind,
        // Separate parameters to reduce tokens in generated class API.
        class_name: &'static str,
        method_name: &'static str,
        object_ptr: sys::GDExtensionObjectPtr,
        maybe_instance_id: Option<InstanceId>, // if not static
        args: Params,
        varargs: &[Variant],
    ) -> CallResult<Ret> {
        let call_ctx = CallContext::outbound(class_name, method_name);
        //$crate::out!("out_class_varcall: {call_ctx}");

        // Note: varcalls are not safe from failing, if they happen through an object pointer -> validity check necessary.
        if let Some(instance_id) = maybe_instance_id {
            crate::classes::ensure_object_alive(instance_id, object_ptr, &call_ctx);
        }

        let class_fn = sys::interface_fn!(object_method_bind_call);

        let variant = args.with_args(|explicit_args, _| {
            let mut variant_ptrs = Vec::with_capacity(explicit_args.len() + varargs.len());
            variant_ptrs.extend(explicit_args.iter().map(Variant::var_sys));
            variant_ptrs.extend(varargs.iter().map(Variant::var_sys));

            Variant::new_with_var_uninit_result(|return_ptr| {
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
            })
        });

        variant.and_then(|v| {
            v.try_to::<Ret>()
                .map_err(|e| CallError::failed_return_conversion::<Ret>(&call_ctx, e))
        })
    }

    #[cfg(since_api = "4.3")]
    pub unsafe fn out_script_virtual_call(
        // Separate parameters to reduce tokens in macro-generated API.
        class_name: &'static str,
        method_name: &'static str,
        method_sname_ptr: sys::GDExtensionConstStringNamePtr,
        object_ptr: sys::GDExtensionObjectPtr,
        args: Params,
    ) -> Ret {
        // Assumes that caller has previously checked existence of a virtual method.

        let call_ctx = CallContext::outbound(class_name, method_name);
        //$crate::out!("out_script_virtual_call: {call_ctx}");

        let object_call_script_method = sys::interface_fn!(object_call_script_method);

        let variant = args.with_args(|_, sys_args| {
            Variant::new_with_var_uninit(|return_ptr| {
                let mut err = sys::default_call_error();
                object_call_script_method(
                    object_ptr,
                    method_sname_ptr,
                    sys_args.as_ptr(),
                    sys_args.len() as i64,
                    return_ptr,
                    std::ptr::addr_of_mut!(err),
                );
            })
        });

        let result = <Ret as FromGodot>::try_from_variant(&variant);
        result.unwrap_or_else(|err| return_error::<Ret>(&call_ctx, err))
    }

    // Note: this is doing a ptrcall, but uses variant conversions for it.
    #[inline]
    pub unsafe fn out_utility_ptrcall_varargs(
        utility_fn: sys::UtilityFunctionBind,
        function_name: &'static str,
        args: Params,
        varargs: &[Variant],
    ) -> Ret {
        let call_ctx = CallContext::outbound("", function_name);
        //$crate::out!("out_utility_ptrcall_varargs: {call_ctx}");

        let result = args.with_ptr_args(|explicit_args| {
            let mut type_ptrs = Vec::with_capacity(explicit_args.len() + varargs.len());
            type_ptrs.extend(explicit_args.iter());
            type_ptrs.extend(varargs.iter().map(sys::GodotFfi::sys));

            // Important: this calls from_sys_init_default().
            new_from_ptrcall::<Ret>(|return_ptr| {
                utility_fn(return_ptr, type_ptrs.as_ptr(), type_ptrs.len() as i32);
            })
        });

        result.unwrap_or_else(|err| return_error::<Ret>(&call_ctx, err))
    }

    #[inline]
    pub unsafe fn out_builtin_ptrcall_varargs(
        builtin_fn: sys::BuiltinMethodBind,
        class_name: &'static str,
        method_name: &'static str,
        type_ptr: sys::GDExtensionTypePtr,
        args: Params,
        varargs: &[Variant],
    ) -> Ret {
        let call_ctx = CallContext::outbound(class_name, method_name);
        //$crate::out!("out_builtin_ptrcall_varargs: {call_ctx}");

        let result = args.with_ptr_args(|explicit_args| {
            let mut type_ptrs = Vec::with_capacity(explicit_args.len() + varargs.len());
            type_ptrs.extend(explicit_args.iter());
            type_ptrs.extend(varargs.iter().map(sys::GodotFfi::sys));

            // Important: this calls from_sys_init_default().
            new_from_ptrcall::<Ret>(|return_ptr| {
                builtin_fn(
                    type_ptr,
                    type_ptrs.as_ptr(),
                    return_ptr,
                    type_ptrs.len() as i32,
                );
            })
        });

        result.unwrap_or_else(|err| return_error::<Ret>(&call_ctx, err))
    }

    #[inline]
    pub unsafe fn out_class_ptrcall(
        method_bind: sys::ClassMethodBind,
        // Separate parameters to reduce tokens in generated class API.
        class_name: &'static str,
        method_name: &'static str,
        object_ptr: sys::GDExtensionObjectPtr,
        maybe_instance_id: Option<InstanceId>, // if not static
        args: Params,
    ) -> Ret {
        let call_ctx = CallContext::outbound(class_name, method_name);
        // $crate::out!("out_class_ptrcall: {call_ctx}");

        if let Some(instance_id) = maybe_instance_id {
            crate::classes::ensure_object_alive(instance_id, object_ptr, &call_ctx);
        }

        let class_fn = sys::interface_fn!(object_method_bind_ptrcall);

        let result = args.with_ptr_args(|explicit_args| {
            new_from_ptrcall::<Ret>(|return_ptr| {
                class_fn(
                    method_bind.0,
                    object_ptr,
                    explicit_args.as_ptr(),
                    return_ptr,
                );
            })
        });

        result.unwrap_or_else(|err| return_error::<Ret>(&call_ctx, err))
    }

    #[inline]
    pub unsafe fn out_builtin_ptrcall(
        builtin_fn: sys::BuiltinMethodBind,
        // Separate parameters to reduce tokens in generated class API.
        class_name: &'static str,
        method_name: &'static str,
        type_ptr: sys::GDExtensionTypePtr,
        args: Params,
    ) -> Ret {
        let call_ctx = CallContext::outbound(class_name, method_name);
        // $crate::out!("out_builtin_ptrcall: {call_ctx}");

        let result = args.with_ptr_args(|explicit_args| {
            new_from_ptrcall::<Ret>(|return_ptr| {
                builtin_fn(
                    type_ptr,
                    explicit_args.as_ptr(),
                    return_ptr,
                    explicit_args.len() as i32,
                );
            })
        });

        result.unwrap_or_else(|err| return_error::<Ret>(&call_ctx, err))
    }

    #[inline]
    pub unsafe fn out_utility_ptrcall(
        utility_fn: sys::UtilityFunctionBind,
        function_name: &'static str,
        args: Params,
    ) -> Ret {
        let call_ctx = CallContext::outbound("", function_name);
        // $crate::out!("out_utility_ptrcall: {call_ctx}");

        let result = args.with_ptr_args(|explicit_args| {
            new_from_ptrcall::<Ret>(|return_ptr| {
                utility_fn(
                    return_ptr,
                    explicit_args.as_ptr(),
                    explicit_args.len() as i32,
                );
            })
        });

        result.unwrap_or_else(|err| return_error::<Ret>(&call_ctx, err))
    }
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
    let val = ret_val.to_godot();
    let ffi = val.into_ffi();

    ffi.move_return_ptr(ret, call_type);
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

    /// Call from Godot into a custom Callable.
    pub fn custom_callable(function_name: &'a str) -> Self {
        Self {
            class_name: Cow::Borrowed("<Callable>"),
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

impl fmt::Display for CallContext<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}::{}", self.class_name, self.function_name)
    }
}
