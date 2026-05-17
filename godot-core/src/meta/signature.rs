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
use crate::meta::error::{CallError, CallResult, ConvertError, ErrorToGodot};
use crate::meta::param_tuple::{LossyTupleFromGodot, TupleFromGodot};
use crate::meta::{
    EngineFromGodot, EngineToGodot, FromGodot, GodotConvert, GodotType, InParamTuple,
    MethodParamOrReturnInfo, OutParamTuple, ParamTuple, ToGodot,
};
use crate::obj::{GodotClass, ValidatedObject};

/// Marker trait for types valid as `#[func]` return values.
///
/// Separates user-facing `#[func]` return types from internal engine types (e.g. `u64`, which implements
/// [`EngineToGodot`] but not [`ToGodot`] and thus not `FuncReturn`).
///
/// Implemented for all [`ToGodot`] types and for `Result<T, E>` where `E: ErrorToGodot<T>`.
#[doc(hidden)]
pub trait FuncReturn: EngineToGodot {}

impl<T: ToGodot> FuncReturn for T {}

impl<T, E> FuncReturn for Result<T, E>
where
    T: ToGodot,
    E: ErrorToGodot<T>,
{
}

/// Checks for `#[func]` expansions that all parameters implement `FromGodot` and the return type implements [`FuncReturn`]
/// (which covers all [`ToGodot`] types and `Result<T, E: ErrorToGodot<T>>`).
///
/// [`Signature`] itself only requires `EngineFromGodot` and `EngineToGodot` for out-calls.
#[inline(always)]
#[doc(hidden)]
pub fn ensure_func_bounds<Params: TupleFromGodot, Ret: FuncReturn>() {}

/// `#[func(lossy)]` analog of [`ensure_func_bounds`]: relaxes per-param bound from [`FromGodot`] to [`EngineFromGodot`], admitting lossy-tier
/// integers (`usize`, `u64`).
///
/// The return bound is just [`EngineToGodot`], not a dedicated `LossyFuncReturn` trait. Reason: `LossyFuncReturn` would be a pure alias —
/// every `ToGodot` type blanket-impls `EngineToGodot`, `Result<T: ToGodot, E: ErrorToGodot<T>>` impls it directly, and lossy-tier integers
/// impl it explicitly. So `EngineToGodot` is already the exact set we want. No new trait needed.
///
/// Out-of-range values surface as `CallError` on the Godot side via the varcall/ptrcall error path (no panic in conversion).
#[inline(always)]
#[doc(hidden)]
pub fn ensure_func_bounds_lossy<Params: LossyTupleFromGodot, Ret: EngineToGodot>() {}

/// A full signature for a function.
///
/// For in-calls (that is, calls from the Godot engine to Rust code) `Params` will implement [`InParamTuple`] and `Ret`
/// will implement [`FuncReturn`] (which covers all [`ToGodot`] types and `Result<T, E: ErrorToGodot<T>>`).
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
impl<Params, Ret> Signature<Params, Ret>
where
    Params: InParamTuple,
    Ret: EngineToGodot,
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

        #[cfg(feature = "itest")]
        trace::push(true, false, call_ctx);

        // SAFETY: TODO.
        let args =
            unsafe { Params::from_varcall_args(args_ptr, arg_count, default_values, call_ctx)? };

        let rust_result = unsafe { func(instance_ptr, args) };
        // SAFETY: TODO.
        unsafe { varcall_return::<Ret>(rust_result, ret, err, call_ctx)? };
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

        #[cfg(feature = "itest")]
        trace::push(true, true, call_ctx);

        // SAFETY: TODO.
        let args = unsafe { Params::from_ptrcall_args(args_ptr, call_type, call_ctx)? };

        // SAFETY:
        // `ret` is always a pointer to an initialized value of type $R
        // TODO: double-check the above
        unsafe { ptrcall_return::<Ret>(func(instance_ptr, args), ret, call_ctx, call_type)? };

        Ok(())
    }
}

/// Out-calls:
///
/// Calls going from Rust code to the Godot engine.
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

        // Silence inbound `#[func]` failure prints during this out-call; caller observes the error via the returned `CallError`.
        let _guard = crate::private::OutCallGuard::new();

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

        // SAFETY: caller guarantees `object_ptr` is a live object with virtual method `method_sname_ptr`.
        let variant =
            unsafe { out_script_virtual_call_inner(&call_ctx, method_sname_ptr, object_ptr, args) };

        let result = <Ret as FromGodot>::try_from_variant(&variant);
        result.unwrap_or_else(|err| return_error_dyn(&call_ctx, std::any::type_name::<Ret>(), err))
    }

    /// Make a script-virtual call that may resolve asynchronously (GDScript `await`).
    ///
    /// Behaves like [`out_script_virtual_call`](Self::out_script_virtual_call), but if the GDScript override uses `await`, the engine returns a
    /// coroutine handle instead of the final value. This function then awaits its completion and converts the eventual result to `Ret`.
    ///
    /// The synchronous engine call (which starts the coroutine) happens immediately; the returned future only keeps the coroutine handle alive,
    /// so it does not borrow the calling object across `.await`.
    ///
    /// # Safety
    /// Same as [`out_script_virtual_call`](Self::out_script_virtual_call).
    #[cfg(since_api = "4.3")]
    pub unsafe fn out_script_virtual_call_async(
        // Separate parameters to reduce tokens in macro-generated API.
        class_name: &'static str,
        method_name: &'static str,
        method_sname_ptr: sys::GDExtensionConstStringNamePtr,
        object_ptr: sys::GDExtensionObjectPtr,
        args: Params,
    ) -> impl std::future::Future<Output = Ret>
    where
        Ret: FromGodot,
    {
        // Assumes that caller has previously checked existence of a virtual method.

        let call_ctx = CallContext::outbound(class_name, method_name);

        // The engine call (which may start a GDScript coroutine) runs eagerly here; only the coroutine handling is deferred to the future.
        // SAFETY: caller guarantees `object_ptr` is a live object with virtual method `method_sname_ptr`.
        let variant =
            unsafe { out_script_virtual_call_inner(&call_ctx, method_sname_ptr, object_ptr, args) };

        resolve_gdscript_coroutine::<Ret>(call_ctx, variant)
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
            pack_ptrcall_args(args, |explicit_args, arg_count| {
                finish_ptrcall(&call_ctx, false, &mut |return_ptr| {
                    let type_ptrs = concat_varargs_ptrs(explicit_args, arg_count, varargs);
                    // Important: this calls from_sys_init_default().
                    // SAFETY: TODO.
                    utility_fn(return_ptr, type_ptrs.as_ptr(), type_ptrs.len() as i32);
                })
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
            pack_ptrcall_args(args, |explicit_args, arg_count| {
                finish_ptrcall(&call_ctx, false, &mut |return_ptr| {
                    let type_ptrs = concat_varargs_ptrs(explicit_args, arg_count, varargs);
                    // Important: this calls from_sys_init_default().
                    builtin_fn(
                        type_ptr,
                        type_ptrs.as_ptr(),
                        return_ptr,
                        type_ptrs.len() as i32,
                    );
                })
            })
        }
    }

    /// Make a ptrcall to the Godot engine for a class method.
    ///
    /// # Safety
    /// - `method_bind` must expect explicit args `args`, and return a value of type `Ret`
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
        let object_ptr = ValidatedObject::object_ptr(validated_obj.as_ref());

        unsafe {
            pack_ptrcall_args(args, |explicit_args, _arg_count| {
                finish_ptrcall(&call_ctx, true, &mut |return_ptr| {
                    dispatch_class_ptrcall(method_bind, object_ptr, explicit_args, return_ptr);
                })
            })
        }
    }

    /// Make a ptrcall to the Godot engine for a builtin method.
    ///
    /// # Safety
    /// - `builtin_fn` must expect explicit args `args`, and return a value of type `Ret`
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
            pack_ptrcall_args(args, |explicit_args, arg_count| {
                finish_ptrcall(&call_ctx, false, &mut |return_ptr| {
                    dispatch_builtin_ptrcall(
                        builtin_fn,
                        type_ptr,
                        explicit_args,
                        arg_count,
                        return_ptr,
                    );
                })
            })
        }
    }

    /// Make a ptrcall to the Godot engine for a utility function.
    ///
    /// # Safety
    /// - `utility_fn` must expect explicit args `args`, and return a value of type `Ret`
    pub unsafe fn out_utility_ptrcall(
        utility_fn: sys::UtilityFunctionBind,
        function_name: &'static str,
        args: Params,
    ) -> Ret {
        let call_ctx = CallContext::outbound("", function_name);
        // $crate::out!("out_utility_ptrcall: {call_ctx}");

        unsafe {
            pack_ptrcall_args(args, |explicit_args, arg_count| {
                finish_ptrcall(&call_ctx, false, &mut |return_ptr| {
                    dispatch_utility_ptrcall(utility_fn, explicit_args, arg_count, return_ptr);
                })
            })
        }
    }
}

/// Packs ptrcall arguments into a pointer/length pair.
///
/// # Safety
/// This forwards the slice produced by [`OutParamTuple::with_type_pointers`]. The callback must not retain the raw pointer beyond the call.
unsafe fn pack_ptrcall_args<Params: OutParamTuple, R>(
    args: Params,
    f: impl FnOnce(*const sys::GDExtensionConstTypePtr, usize) -> R,
) -> R {
    args.with_type_pointers(|explicit_args| f(explicit_args.as_ptr(), explicit_args.len()))
}

/// Concatenates explicit ptrcall args with `varargs` into a single pointer vector.
///
/// # Safety
/// `explicit_args` must point to an array of at least `arg_count` valid `GDExtensionConstTypePtr` entries. Pointers in `varargs` must
/// remain valid for the duration of the returned vector's use.
unsafe fn concat_varargs_ptrs(
    explicit_args: *const sys::GDExtensionConstTypePtr,
    arg_count: usize,
    varargs: &[Variant],
) -> Vec<sys::GDExtensionConstTypePtr> {
    let explicit_args = unsafe { std::slice::from_raw_parts(explicit_args, arg_count) };
    let mut type_ptrs = Vec::with_capacity(explicit_args.len() + varargs.len());
    type_ptrs.extend(explicit_args.iter().copied());
    type_ptrs.extend(varargs.iter().map(sys::GodotFfi::sys));
    type_ptrs
}

/// Performs the return-value initialization and decode for a ptrcall.
///
/// `init` uses dynamic (`&mut dyn FnMut`) instead of static (`impl FnOnce`) dispatch on purpose: each call site captures different
/// parameters, which makes the closure type unique. With static dispatch, `finish_ptrcall<Ret, F>` would be monomorphized once per
/// `(Ret, captured-closure-type)` pair -- at last measurement ~700 instances despite only ~80 distinct `Ret` types. Switching `init`
/// to `dyn` collapses that to ~180 instances (one per `Ret` that escapes the trait object), cutting godot-core LLVM IR by ~8% and
/// debug compile time by ~25%, with no measurable runtime regression: the dyn fat-pointer call adds 1 indirect on a path that already
/// crosses the GDExtension FFI boundary, where Godot's own dispatch dominates.
///
/// Tried but rejected: extending the same trick to `pack_ptrcall_args` / `with_type_pointers`. Doing so requires call sites to
/// capture `Ret` via `Option<Ret>`/`MaybeUninit`, and the per-site option dance generates more code than the helper-outlining saves.
///
/// # Safety
/// This calls [`GodotFfi::new_with_init`] and passes the return pointer to `init`, see that function for safety docs.
unsafe fn finish_ptrcall<Ret: EngineFromGodot>(
    call_ctx: &CallContext,
    is_class_method: bool,
    init: &mut dyn FnMut(sys::GDExtensionTypePtr),
) -> Ret {
    let mut ffi = unsafe { <<Ret::Via as GodotType>::Ffi>::new_with_init(init) };

    // Class methods returning static type Object (e.g. `EditorProperty::get_edited_object()`) hand back a raw `Object*` without incrementing the
    // reference count. If the runtime type is `RefCounted`, we need to do that ourselves. Builtin/utility methods (e.g. `Callable::get_object()`)
    // already return an owning `Ref<T>`, so they are not touched. See also https://github.com/godot-rust/gdext/issues/1626.
    if is_class_method {
        ffi.adjust_refcount_on_ptrcall_return();
    }

    match Ret::Via::try_from_ffi(ffi).and_then(Ret::engine_try_from_godot) {
        Ok(ret) => ret,
        Err(err) => return_error_dyn(call_ctx, std::any::type_name::<Ret>(), err),
    }
}

#[inline(never)]
unsafe fn dispatch_class_ptrcall(
    method_bind: sys::ClassMethodBind,
    object_ptr: sys::GDExtensionObjectPtr,
    args_ptr: *const sys::GDExtensionConstTypePtr,
    return_ptr: sys::GDExtensionTypePtr,
) {
    unsafe {
        sys::interface_fn!(object_method_bind_ptrcall)(
            method_bind.0,
            object_ptr,
            args_ptr,
            return_ptr,
        )
    };
}

#[inline(never)]
unsafe fn dispatch_builtin_ptrcall(
    builtin_fn: sys::BuiltinMethodBind,
    type_ptr: sys::GDExtensionTypePtr,
    args_ptr: *const sys::GDExtensionConstTypePtr,
    arg_count: usize,
    return_ptr: sys::GDExtensionTypePtr,
) {
    unsafe { builtin_fn(type_ptr, args_ptr, return_ptr, arg_count as i32) };
}

#[inline(never)]
unsafe fn dispatch_utility_ptrcall(
    utility_fn: sys::UtilityFunctionBind,
    args_ptr: *const sys::GDExtensionConstTypePtr,
    arg_count: usize,
    return_ptr: sys::GDExtensionTypePtr,
) {
    unsafe { utility_fn(return_ptr, args_ptr, arg_count as i32) };
}

/// Moves `ret_val` into `ret`.
///
/// Uses [`EngineToGodot::engine_try_into_variant`] to support `Result<T, E>` without panicking.
///
/// # Safety
/// - `ret` must be a pointer to an initialized `Variant`.
/// - It must be safe to write a `Variant` once to `ret`.
/// - It must be safe to write a `sys::GDExtensionCallError` once to `err`.
unsafe fn varcall_return<R: EngineToGodot>(
    ret_val: R,
    ret: sys::GDExtensionVariantPtr,
    err: *mut sys::GDExtensionCallError,
    call_ctx: &CallContext,
) -> CallResult<()> {
    let ret_variant = ret_val.engine_try_into_variant(call_ctx)?;

    unsafe {
        *(ret as *mut Variant) = ret_variant;
        (*err).error = sys::GDEXTENSION_CALL_OK;
    }

    Ok(())
}

/// Moves `ret_val` into `ret`, if it is `Ok(...)`. Otherwise sets an error.
///
/// # Safety
/// See [`varcall_return`].
pub(crate) unsafe fn varcall_return_checked<R: ToGodot>(
    ret_val: Result<R, ()>, // TODO Err should be custom CallError enum
    ret: sys::GDExtensionVariantPtr,
    err: *mut sys::GDExtensionCallError,
    call_ctx: &CallContext,
) -> CallResult<()> {
    if let Ok(ret_val) = ret_val {
        unsafe { varcall_return(ret_val, ret, err, call_ctx)? };
    } else {
        unsafe {
            *err = sys::default_call_error();
            (*err).error = sys::GDEXTENSION_CALL_ERROR_INVALID_ARGUMENT;
        }
    }
    Ok(())
}

/// Moves `ret_val` into `ret`.
///
/// Uses [`EngineToGodot::engine_try_into_godot_owned`] to check for unexpected (call-failing) `Result<T, E>` errors
/// and produce the `Via` value in one consuming step. On error, returns `Err(CallError)` without writing to the return pointer.
///
/// Note that on the FFI level, ptrcalls have no `r_error` output parameter, so `Result<T, E>` resulting in failed calls (e.g. through
/// `strat::Unexpected`) can't abort the calling GDScript function. Instead, this results in a Godot error print + default value.
///
/// # Safety
/// `ret_val`, `ret`, and `call_type` must follow the safety requirements as laid out in
/// [`GodotFuncMarshal::try_return`](sys::GodotFuncMarshal::try_return).
unsafe fn ptrcall_return<R: EngineToGodot>(
    ret_val: R,
    ret: sys::GDExtensionTypePtr,
    call_ctx: &CallContext,
    call_type: sys::PtrcallType,
) -> CallResult<()> {
    // Consumes ret_val, checks for call-failing outcomes, and produces the Via value directly.
    // For non-Result types this is a no-op (always Ok) equivalent to engine_to_godot_owned().
    let val = ret_val.engine_try_into_godot_owned(call_ctx)?;

    unsafe {
        let ffi = val.into_ffi();
        ffi.move_return_ptr(ret, call_type);
    }

    Ok(())
}

#[cold]
#[inline(never)]
fn return_error_dyn(call_ctx: &CallContext, return_ty: &'static str, err: ConvertError) -> ! {
    panic!("in function `{call_ctx}` at return type {return_ty}: {err}");
}

/// Shared engine call for [`Signature::out_script_virtual_call`] and its async variant: performs the `object_call_script_method` varcall and
/// surfaces a Godot-side call error as a panic. Returns the raw `Variant` result (a coroutine handle if the GDScript override used `await`).
///
/// # Safety
/// `object_ptr` must be a live object with virtual method `method_sname_ptr`, and `args` must match that method's signature.
#[cfg(since_api = "4.3")]
unsafe fn out_script_virtual_call_inner<Params: OutParamTuple>(
    call_ctx: &CallContext,
    method_sname_ptr: sys::GDExtensionConstStringNamePtr,
    object_ptr: sys::GDExtensionObjectPtr,
    args: Params,
) -> Variant {
    let object_call_script_method = sys::interface_fn!(object_call_script_method);

    let variant = args.with_variants(|call_args| {
        let variant_ptrs: Vec<_> = call_args.iter().map(Variant::var_sys).collect();

        // SAFETY: `object_ptr`/`method_sname_ptr` and the argument pointers are valid per the caller's guarantee; `return_ptr` is uninitialized
        // result storage that the engine writes on success.
        unsafe {
            Variant::new_with_var_uninit_result(|return_ptr| {
                let mut err = sys::default_call_error();
                object_call_script_method(
                    object_ptr,
                    method_sname_ptr,
                    variant_ptrs.as_ptr(),
                    variant_ptrs.len() as i64,
                    return_ptr,
                    &raw mut err,
                );

                CallError::check_out_varcall(call_ctx, err, call_args, &[] as &[Variant])
            })
        }
    });

    variant.unwrap_or_else(|err| panic!("{err}"))
}

/// Converts the return value of a script-virtual call, awaiting completion if the GDScript override used `await`.
///
/// A GDScript function that uses `await` returns a `GDScriptFunctionState` object whose `completed` signal eventually carries the real return
/// value. That type is not exposed in the GDExtension API, so it is detected by its class name.
pub(crate) async fn resolve_gdscript_coroutine<Ret: FromGodot>(
    call_ctx: CallContext<'static>,
    variant: Variant,
) -> Ret {
    use crate::builtin::Signal;
    use crate::classes::Object;
    use crate::obj::Gd;

    // `get_class()` (engine method) is used instead of `Gd::dynamic_class()`: the latter is backed by `object_get_class_name`, which only
    // reports exposed extension classes and returns "RefCounted" for the hidden `GDScriptFunctionState`. `get_class()` returns the real name.
    let result = if let Ok(state) = variant.try_to::<Gd<Object>>()
        && state.get_class() == "GDScriptFunctionState"
    {
        let signal = Signal::from_object_signal(&state, "completed");
        let (result,) = signal.to_future::<(Variant,)>().await;
        // `state` is kept alive across the await point, so the coroutine isn't dropped before completion.
        drop(state);
        result
    } else {
        variant
    };

    <Ret as FromGodot>::try_from_variant(&result)
        .unwrap_or_else(|err| return_error_dyn(&call_ctx, std::any::type_name::<Ret>(), err))
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

#[cfg(feature = "itest")]
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
