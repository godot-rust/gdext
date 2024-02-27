/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::builtin::meta::{CallContext, ConvertError, ToGodot};
use crate::builtin::Variant;
use crate::sys;
use godot_ffi::{join_debug, VariantType};
use std::error::Error;
use std::fmt;

/// Error capable of representing failed function calls.
///
/// This type is returned from _varcall_ functions in the Godot API that begin with `try_` prefixes,
/// e.g. [`Object::try_call()`](crate::engine::Object::try_call) or [`Node::try_rpc()`](crate::engine::Node::try_rpc).
///
/// Allows to inspect the involved class and method via `class_name()` and `method_name()`. Implements the `std::error::Error` trait, so
/// it comes with `Display` and `Error::source()` APIs.
///
/// # Possible error causes
/// Several reasons can cause a function call to fail. The reason is described in the `Display` impl.
///
/// - **Invalid method**: The method does not exist on the object.
/// - **Failed argument conversion**: The arguments passed to the method cannot be converted to the declared parameter types.
/// - **Failed return value conversion**: The return value of a dynamic method (`Variant`) cannot be converted to the expected return type.
/// - **Too many or too few arguments**: The number of arguments passed to the method does not match the number of parameters.
/// - **User panic**: A Rust method caused a panic.
///
/// # Chained errors
/// Let's say you have this code, and you want to call the method dynamically with `Object::try_call()`.
///
/// Then, the immediate `CallError` will refer to the `Object::try_call` method, and its source will refer to `MyClass::my_method`
/// (the actual method that failed).
/// ```no_run
/// use godot::prelude::*;
/// use std::error::Error;
/// # use godot_core::builtin::meta::CallError;
/// #[derive(GodotClass)]
/// # #[class(init)]
/// struct MyClass;
///
/// #[godot_api]
/// impl MyClass {
///     #[func]
///     fn my_method(&self, arg: i64) {}
/// }
///
/// fn some_method() {
///     let obj: Gd<MyClass> = MyClass::new_gd();
///
///     // Dynamic call. Note: forgot to pass the argument.
///     let result: Result<Variant, CallError> = obj.try_call("my_method", &[]);
///
///     // Get immediate and original errors. Note that source() can be None or have type ConvertError.
///     let outer: CallError = result.unwrap_err();
///     let inner: CallError = outer.source().downcast_ref::<CallError>().unwrap();
/// }
#[derive(Debug)]
pub struct CallError {
    class_name: String,
    function_name: String,
    call_expr: String,
    reason: String,
    source: Option<SourceError>,
}

impl CallError {
    // Naming:
    // - check_* means possible failure -- Result<(), Self> is returned.
    // - failed_* means definitive failure -- Self is returned.

    /// Name of the class/builtin whose method failed. **Not** the dynamic type.
    ///
    /// Returns `None` if this is a utility function (without a surrounding class/builtin).
    ///
    /// This is the static and not the dynamic type. For example, if you invoke `call()` on a `Gd<Node>`, you are effectively invoking
    /// `Object::call()` (through `DerefMut`), and the class name will be `Object`.
    pub fn class_name(&self) -> Option<&str> {
        if self.class_name.is_empty() {
            None
        } else {
            Some(&self.class_name)
        }
    }

    /// Name of the function or method that failed.
    pub fn method_name(&self) -> &str {
        &self.function_name
    }

    /// Describes the error.
    ///
    /// This is the same as the `Display`/`ToString` repr, but without the prefix mentioning that this is a function call error,
    /// and without any source error information.
    fn message(&self, with_source: bool) -> String {
        let Self {
            call_expr, reason, ..
        } = self;

        let reason_str = if reason.is_empty() {
            String::new()
        } else {
            format!("\n  Reason: {reason}")
        };

        // let source_str = if with_source {
        //     self.source()
        //         .map(|e| format!("\n  Source: {}", e))
        //         .unwrap_or_default()
        // } else {
        //     String::new()
        // };

        let source_str = match &self.source {
            Some(SourceError::Convert(e)) if with_source => format!("\n  Source: {}", e),
            Some(SourceError::Call(e)) if with_source => format!("\n  Source: {}", e.message(true)),
            _ => String::new(),
        };

        format!("{call_expr}{reason_str}{source_str}")
    }

    /// Checks whether number of arguments matches the number of parameters.
    pub(crate) fn check_arg_count(
        call_ctx: &CallContext,
        arg_count: i64,
        param_count: i64,
    ) -> Result<(), Self> {
        // This will need to be adjusted once optional parameters are supported in #[func].
        if arg_count == param_count {
            return Ok(());
        }

        let param_plural = plural(param_count);
        let arg_plural = plural(arg_count);

        Err(Self::new(
            call_ctx,
            format!(
                "function has {param_count} parameter{param_plural}, but received {arg_count} argument{arg_plural}"
            ),
            None,
        ))
    }

    /// Checks the Godot side of a varcall (low-level `sys::GDExtensionCallError`).
    pub(crate) fn check_out_varcall<T: ToGodot>(
        call_ctx: &CallContext,
        err: sys::GDExtensionCallError,
        explicit_args: &[T],
        varargs: &[Variant],
    ) -> Result<(), Self> {
        if err.error == sys::GDEXTENSION_CALL_OK {
            return Ok(());
        }

        let mut arg_types = Vec::with_capacity(explicit_args.len() + varargs.len());
        arg_types.extend(explicit_args.iter().map(|arg| arg.to_variant().get_type()));
        arg_types.extend(varargs.iter().map(Variant::get_type));

        let explicit_args_str = join_args(explicit_args.iter().map(|arg| arg.to_variant()));
        let vararg_str = if varargs.is_empty() {
            String::new()
        } else {
            format!(", varargs {}", join_args(varargs.into_iter().cloned()))
        };

        let call_expr = format!("{call_ctx}({explicit_args_str}{vararg_str})");

        // If the call error encodes an error generated by us, decode it.
        let mut source_error = None;
        if err.error == sys::GODOT_RUST_CUSTOM_CALL_ERROR {
            source_error = crate::private::call_error_remove(&err) //.
                .map(|e| SourceError::Call(Box::new(e)));
        }

        Err(Self::failed_varcall_inner(
            call_ctx,
            call_expr,
            err,
            &arg_types,
            source_error,
        ))
    }

    /// Returns an error for a failed parameter conversion.
    pub(crate) fn failed_param_conversion<P>(
        call_ctx: &CallContext,
        param_index: isize,
        convert_error: ConvertError,
    ) -> Self {
        let param_ty = std::any::type_name::<P>();

        Self::new(
            call_ctx,
            format!("parameter [{param_index}] of type {param_ty} failed to convert to Variant; {convert_error}"),
            Some(convert_error),
        )
    }

    /// Returns an error for a failed return type conversion.
    pub(crate) fn failed_return_conversion<R>(
        call_ctx: &CallContext,
        convert_error: ConvertError,
    ) -> Self {
        let return_ty = std::any::type_name::<R>();

        Self::new(
            call_ctx,
            format!("return type {return_ty} failed to convert from Variant; {convert_error}"),
            Some(convert_error),
        )
    }

    fn failed_varcall_inner(
        call_ctx: &CallContext,
        call_expr: String,
        err: sys::GDExtensionCallError,
        arg_types: &[VariantType],
        source: Option<SourceError>,
    ) -> Self {
        // This specializes on reflection-style calls, e.g. call(), rpc() etc.
        // In these cases, varargs are the _actual_ arguments, with required args being metadata such as method name.

        debug_assert_ne!(err.error, sys::GDEXTENSION_CALL_OK); // already checked outside

        let sys::GDExtensionCallError {
            error,
            argument,
            expected,
        } = err;

        let argc = arg_types.len();
        let reason = match error {
            sys::GDEXTENSION_CALL_ERROR_INVALID_METHOD => "method not found".to_string(),
            sys::GDEXTENSION_CALL_ERROR_INVALID_ARGUMENT => {
                let from = arg_types[argument as usize];
                let to = VariantType::from_sys(expected as sys::GDExtensionVariantType);
                let i = argument + 1;

                format!("cannot convert argument #{i} from {from:?} to {to:?}")
            }
            sys::GDEXTENSION_CALL_ERROR_TOO_MANY_ARGUMENTS => {
                format!("too many arguments; expected {argument}, but called with {argc}")
            }
            sys::GDEXTENSION_CALL_ERROR_TOO_FEW_ARGUMENTS => {
                format!("too few arguments; expected {argument}, but called with {argc}")
            }
            sys::GDEXTENSION_CALL_ERROR_INSTANCE_IS_NULL => "instance is null".to_string(),
            sys::GDEXTENSION_CALL_ERROR_METHOD_NOT_CONST => "method is not const".to_string(), // not handled in Godot
            sys::GODOT_RUST_CUSTOM_CALL_ERROR => String::new(),
            _ => format!("unknown reason (error code {error})"),
        };

        Self {
            class_name: call_ctx.class_name.to_string(),
            function_name: call_ctx.function_name.to_string(),
            call_expr,
            reason,
            source,
        }
    }

    #[doc(hidden)]
    pub fn failed_by_user_panic(call_ctx: &CallContext, reason: String) -> Self {
        Self::new(call_ctx, reason, None)
    }

    fn new(call_ctx: &CallContext, reason: String, source: Option<ConvertError>) -> Self {
        Self {
            class_name: call_ctx.class_name.to_string(),
            function_name: call_ctx.function_name.to_string(),
            call_expr: format!("{call_ctx}()"),
            reason,
            source: source.map(|e| SourceError::Convert(Box::new(e))),
        }
    }
}

impl fmt::Display for CallError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "godot-rust function call failed: {}", self.message(true))
    }
}

impl Error for CallError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self.source.as_ref() {
            Some(SourceError::Convert(e)) => deref_to::<ConvertError>(e),
            Some(SourceError::Call(e)) => deref_to::<CallError>(e),
            None => None,
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Implementation

#[derive(Debug)]
enum SourceError {
    Convert(Box<ConvertError>),
    Call(Box<CallError>),
}

/// Explicit dereferencing to a certain type. Avoids accidentally returning `&Box<T>` or so.
fn deref_to<T>(t: &T) -> Option<&(dyn Error + 'static)>
where
    T: Error + 'static,
{
    Some(t)
}

fn join_args(args: impl Iterator<Item = Variant>) -> String {
    join_debug(args)
}

fn plural(count: i64) -> &'static str {
    if count == 1 {
        ""
    } else {
        "s"
    }
}
