/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::borrow::Cow;
use std::fmt;
use std::marker::PhantomData;

use godot_ffi as sys;
use sys::GodotFfi;

use crate::builtin::Variant;
use crate::meta::error::{CallError, CallResult, ConvertError};
use crate::meta::{
    EngineFromGodot, EngineToGodot, FromGodot, GodotConvert, GodotType, InParamTuple,
    MethodParamOrReturnInfo, OutParamTuple, ParamTuple, ToGodot, TupleFromGodot,
};
use crate::obj::{GodotClass, ValidatedObject};

/// Checks for `#[func]` expansions that all parameters implement `FromGodot` and the return type implements `ToGodot`.
///
/// [`Signature`] itself only requires `EngineFromGodot` and `EngineToGodot`.
#[inline(always)]
#[doc(hidden)]
pub fn ensure_func_bounds<Params: TupleFromGodot, Ret: ToGodot>() {}

/// A full signature for a function.
///
/// For in-calls (that is, calls from the Godot engine to Rust code) `Params` will implement [`InParamTuple`] and `Ret`
/// will implement [`ToGodot`].
///
/// For out-calls (that is calls from Rust code to the Godot engine) `Params` will implement [`OutParamTuple`] and `Ret`
/// will implement [`FromGodot`].
#[doc(hidden)] // Hidden since v0.3.2.
pub struct Signature<Params, Ret> {
    _p: PhantomData<Params>,
    _r: PhantomData<Ret>,
}

impl<Params: ParamTuple, Ret: GodotConvert> Signature<Params, Ret> {
    pub fn param_names(param_names: &[&str]) -> Vec<MethodParamOrReturnInfo> {
        assert_eq!(
            param_names.len(),
            Params::LEN,
            "`param_names` should contain one name for each parameter"
        );

        param_names
            .iter()
            .enumerate()
            .map(|(index, param_name)| Params::param_info(index, param_name).unwrap())
            .collect()
    }
}

/// In-calls (varcall):
///
/// Calls going from the Godot engine to Rust code, using varcall (for user `#[func]` methods with varargs/defaults).
#[deny(unsafe_op_in_unsafe_fn)]
impl<Params, Ret> Signature<Params, Ret>
where
    Params: InParamTuple,
    Ret: EngineToGodot<Via: Clone>,
{
    /// Receive a varcall from Godot, and return the value in `ret` as a variant pointer.
    ///
    /// # Safety
    /// A call to this function must be caused by Godot making a varcall with parameters `Params` and return type `Ret`.
    #[inline]
    #[allow(clippy::too_many_arguments)]
    pub unsafe fn in_varcall(
        instance_ptr: sys::GDExtensionClassInstancePtr,
        call_ctx: &CallContext,
        args_ptr: *const sys::GDExtensionConstVariantPtr,
        arg_count: i64,
        default_values: &[Variant],
        ret: sys::GDExtensionVariantPtr,
        err: *mut sys::GDExtensionCallError,
        func: unsafe fn(sys::GDExtensionClassInstancePtr, Params) -> Ret,
    ) -> CallResult<()> {
        //$crate::out!("in_varcall: {call_ctx}");
        let arg_count = arg_count as usize;
        CallError::check_arg_count(call_ctx, arg_count, default_values.len(), Params::LEN)?;

        #[cfg(feature = "trace")]
        trace::push(true, false, call_ctx);

        // SAFETY: TODO.
        let args =
            unsafe { Params::from_varcall_args(args_ptr, arg_count, default_values, call_ctx)? };

        let rust_result = unsafe { func(instance_ptr, args) };
        // SAFETY: TODO.
        unsafe { varcall_return::<Ret>(rust_result, ret, err) };
        Ok(())
    }

    /// Receive a ptrcall from Godot, and return the value in `ret` as a type pointer.
    ///
    /// # Safety
    ///
    /// A call to this function must be caused by Godot making a ptrcall with parameters `Params` and return type `Ret`.
    #[inline]
    pub unsafe fn in_ptrcall(
        instance_ptr: sys::GDExtensionClassInstancePtr,
        call_ctx: &CallContext,
        args_ptr: *const sys::GDExtensionConstTypePtr,
        ret: sys::GDExtensionTypePtr,
        func: fn(sys::GDExtensionClassInstancePtr, Params) -> Ret,
        call_type: sys::PtrcallType,
    ) -> CallResult<()> {
        // $crate::out!("in_ptrcall: {call_ctx}");

        #[cfg(feature = "trace")]
        trace::push(true, true, call_ctx);

        // SAFETY: TODO.
        let args = unsafe { Params::from_ptrcall_args(args_ptr, call_type, call_ctx)? };

        // SAFETY:
        // `ret` is always a pointer to an initialized value of type $R
        // TODO: double-check the above
        unsafe { ptrcall_return::<Ret>(func(instance_ptr, args), ret, call_ctx, call_type) };

        Ok(())
    }
}

/// Out-calls:
///
/// Calls going from Rust code to the Godot engine.
#[deny(unsafe_op_in_unsafe_fn)]
impl<Params: OutParamTuple, Ret: EngineFromGodot> Signature<Params, Ret> {
    /// Make a varcall to the Godot engine for a class method.
    ///
    /// # Safety
    /// - `method_bind` must expect explicit args `args`, varargs `varargs`, and return a value of type `Ret`
    #[inline]
    pub unsafe fn out_class_varcall(
        method_bind: sys::ClassMethodBind,
        // Separate parameters to reduce tokens in generated class API.
        class_name: &'static str,
        method_name: &'static str,
        validated_obj: Option<ValidatedObject>,
        args: Params,
        varargs: &[Variant],
    ) -> CallResult<Ret> {
        let call_ctx = CallContext::outbound(class_name, method_name);
        //$crate::out!("out_class_varcall: {call_ctx}");

        let class_fn = sys::interface_fn!(object_method_bind_call);

        let variant = args.with_variants(|explicit_args| {
            let mut variant_ptrs = Vec::with_capacity(explicit_args.len() + varargs.len());
            variant_ptrs.extend(explicit_args.iter().map(Variant::var_sys));
            variant_ptrs.extend(varargs.iter().map(Variant::var_sys));

            unsafe {
                Variant::new_with_var_uninit_result(|return_ptr| {
                    let mut err = sys::default_call_error();
                    class_fn(
                        method_bind.0,
                        ValidatedObject::object_ptr(validated_obj.as_ref()),
                        variant_ptrs.as_ptr(),
                        variant_ptrs.len() as i64,
                        return_ptr,
                        &raw mut err,
                    );

                    CallError::check_out_varcall(&call_ctx, err, explicit_args, varargs)
                })
            }
        });

        variant.and_then(|v| {
            Ret::engine_try_from_variant(&v)
                .map_err(|e| CallError::failed_return_conversion::<Ret>(&call_ctx, e))
        })
    }

    /// Make a varcall to the Godot engine for a virtual function call.
    ///
    /// # Safety
    /// - `object_ptr` must be a live instance of a class with a method named `method_sname_ptr`
    /// - The method must expect args `args`, and return a value of type `Ret`
    #[cfg(since_api = "4.3")]
    #[inline]
    pub unsafe fn out_script_virtual_call(
        // Separate parameters to reduce tokens in macro-generated API.
        class_name: &'static str,
        method_name: &'static str,
        method_sname_ptr: sys::GDExtensionConstStringNamePtr,
        object_ptr: sys::GDExtensionObjectPtr,
        args: Params,
    ) -> Ret
    where
        Ret: FromGodot, // FromGodot and not just EngineFromGodot, because script-virtual functions are user-defined.
    {
        // Assumes that caller has previously checked existence of a virtual method.

        let call_ctx = CallContext::outbound(class_name, method_name);
        //$crate::out!("out_script_virtual_call: {call_ctx}");

        let object_call_script_method = sys::interface_fn!(object_call_script_method);

        let variant = args.with_variant_pointers(|sys_args| {
            // SAFETY: TODO.
            unsafe {
                Variant::new_with_var_uninit(|return_ptr| {
                    let mut err = sys::default_call_error();
                    object_call_script_method(
                        object_ptr,
                        method_sname_ptr,
                        sys_args.as_ptr(),
                        sys_args.len() as i64,
                        return_ptr,
                        &raw mut err,
                    );
                })
            }
        });

        let result = <Ret as FromGodot>::try_from_variant(&variant);
        result.unwrap_or_else(|err| return_error::<Ret>(&call_ctx, err))
    }

    /// Make a ptrcall to the Godot engine for a utility function that has varargs.
    ///
    /// # Safety
    /// - `utility_fn` must expect args `args`, varargs `varargs`, and return a value of type `Ret`
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

        unsafe {
            Self::raw_ptrcall(args, &call_ctx, |explicit_args, return_ptr| {
                let mut type_ptrs = Vec::with_capacity(explicit_args.len() + varargs.len());
                type_ptrs.extend(explicit_args.iter());
                type_ptrs.extend(varargs.iter().map(sys::GodotFfi::sys));

                // Important: this calls from_sys_init_default().
                // SAFETY: TODO.
                utility_fn(return_ptr, type_ptrs.as_ptr(), type_ptrs.len() as i32);
            })
        }
    }

    /// Make a ptrcall to the Godot engine for a builtin method that has varargs.
    ///
    /// # Safety
    /// - `builtin_fn` must expect args `args`, varargs `varargs`, and return a value of type `Ret`
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

        unsafe {
            Self::raw_ptrcall(args, &call_ctx, |explicit_args, return_ptr| {
                let mut type_ptrs = Vec::with_capacity(explicit_args.len() + varargs.len());
                type_ptrs.extend(explicit_args.iter());
                type_ptrs.extend(varargs.iter().map(sys::GodotFfi::sys));

                // Important: this calls from_sys_init_default().
                builtin_fn(
                    type_ptr,
                    type_ptrs.as_ptr(),
                    return_ptr,
                    type_ptrs.len() as i32,
                );
            })
        }
    }

    /// Make a ptrcall to the Godot engine for a class method.
    ///
    /// # Safety
    /// - `method_bind` must expect explicit args `args`, and return a value of type `Ret`
    #[inline]
    pub unsafe fn out_class_ptrcall(
        method_bind: sys::ClassMethodBind,
        // Separate parameters to reduce tokens in generated class API.
        class_name: &'static str,
        method_name: &'static str,
        validated_obj: Option<ValidatedObject>,
        args: Params,
    ) -> Ret {
        let call_ctx = CallContext::outbound(class_name, method_name);
        // $crate::out!("out_class_ptrcall: {call_ctx}");

        let class_fn = sys::interface_fn!(object_method_bind_ptrcall);

        unsafe {
            Self::raw_ptrcall(args, &call_ctx, |explicit_args, return_ptr| {
                class_fn(
                    method_bind.0,
                    ValidatedObject::object_ptr(validated_obj.as_ref()),
                    explicit_args.as_ptr(),
                    return_ptr,
                );
            })
        }
    }

    /// Make a ptrcall to the Godot engine for a builtin method.
    ///
    /// # Safety
    /// - `builtin_fn` must expect explicit args `args`, and return a value of type `Ret`
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

        unsafe {
            Self::raw_ptrcall(args, &call_ctx, |explicit_args, return_ptr| {
                builtin_fn(
                    type_ptr,
                    explicit_args.as_ptr(),
                    return_ptr,
                    explicit_args.len() as i32,
                );
            })
        }
    }

    /// Make a ptrcall to the Godot engine for a utility function.
    ///
    /// # Safety
    /// - `utility_fn` must expect explicit args `args`, and return a value of type `Ret`
    #[inline]
    pub unsafe fn out_utility_ptrcall(
        utility_fn: sys::UtilityFunctionBind,
        function_name: &'static str,
        args: Params,
    ) -> Ret {
        let call_ctx = CallContext::outbound("", function_name);
        // $crate::out!("out_utility_ptrcall: {call_ctx}");

        unsafe {
            Self::raw_ptrcall(args, &call_ctx, |explicit_args, return_ptr| {
                utility_fn(
                    return_ptr,
                    explicit_args.as_ptr(),
                    explicit_args.len() as i32,
                );
            })
        }
    }

    /// Performs a ptrcall and processes the return value to give nice error output.
    ///
    /// # Safety
    /// This calls [`GodotFfi::new_with_init`] and passes the ptr as the second argument to `f`, see that function for safety docs.
    unsafe fn raw_ptrcall(
        args: Params,
        call_ctx: &CallContext,
        f: impl FnOnce(&[sys::GDExtensionConstTypePtr], sys::GDExtensionTypePtr),
    ) -> Ret {
        let ffi = args.with_type_pointers(|explicit_args| unsafe {
            <<Ret::Via as GodotType>::Ffi>::new_with_init(|return_ptr| f(explicit_args, return_ptr))
        });

        Ret::Via::try_from_ffi(ffi)
            .and_then(Ret::engine_try_from_godot)
            .unwrap_or_else(|err| return_error::<Ret>(call_ctx, err))
    }
}

/// Moves `ret_val` into `ret`.
///
/// # Safety
/// - `ret` must be a pointer to an initialized `Variant`.
/// - It must be safe to write a `Variant` once to `ret`.
/// - It must be safe to write a `sys::GDExtensionCallError` once to `err`.
unsafe fn varcall_return<R: EngineToGodot>(
    ret_val: R,
    ret: sys::GDExtensionVariantPtr,
    err: *mut sys::GDExtensionCallError,
) {
    let ret_variant = ret_val.engine_to_variant();
    *(ret as *mut Variant) = ret_variant;
    (*err).error = sys::GDEXTENSION_CALL_OK;
}

/// Moves `ret_val` into `ret`, if it is `Ok(...)`. Otherwise sets an error.
///
/// # Safety
/// See [`varcall_return`].
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

/// Moves `ret_val` into `ret`.
///
/// # Safety
/// `ret_val`, `ret`, and `call_type` must follow the safety requirements as laid out in
/// [`GodotFuncMarshal::try_return`](sys::GodotFuncMarshal::try_return).
unsafe fn ptrcall_return<R: EngineToGodot<Via: Clone>>(
    ret_val: R,
    ret: sys::GDExtensionTypePtr,
    _call_ctx: &CallContext,
    call_type: sys::PtrcallType,
) {
    // Needs a value (no ref) to be moved; can't use engine_to_godot() + to_ffi().
    let val = ret_val.engine_to_godot_owned();
    let ffi = val.into_ffi();

    ffi.move_return_ptr(ret, call_type);
}

fn return_error<R>(call_ctx: &CallContext, err: ConvertError) -> ! {
    let return_ty = std::any::type_name::<R>();
    panic!("in function `{call_ctx}` at return type {return_ty}: {err}");
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
    pub const fn custom_callable(function_name: &'a str) -> Self {
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
            class_name: T::class_id().to_cow_str(),
            function_name,
        }
    }
}

impl fmt::Display for CallContext<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}::{}", self.class_name, self.function_name)
    }
}

#[cfg(feature = "trace")]
pub mod trace {
    use std::cell::Cell;

    use crate::meta::CallContext;

    /// Stores information about the current call for diagnostic purposes.
    pub struct CallReport {
        pub class: String,
        pub method: String,
        pub is_inbound: bool,
        pub is_ptrcall: bool,
    }

    pub fn pop() -> CallReport {
        let lock = TRACE.take();
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

        TRACE.set(Some(report));
    }

    thread_local! {
        static TRACE: Cell<Option<CallReport>> = Cell::default();
    }
}
