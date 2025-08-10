/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::error::Error;
use std::fmt;

use godot_ffi::join_debug;

use crate::builtin::{Variant, VariantType};
use crate::meta::error::{ConvertError, ErasedConvertError};
use crate::meta::{CallContext, ToGodot};
use crate::private::PanicPayload;
use crate::sys;

/// Error capable of representing failed function calls.
///
/// This type is returned from _varcall_ functions in the Godot API that begin with `try_` prefixes,
/// e.g. [`Object::try_call()`](crate::classes::Object::try_call) or [`Node::try_rpc()`](crate::classes::Node::try_rpc).
/// _Varcall_ refers to the "variant call" calling convention, meaning that arguments and return values are passed as `Variant` (as opposed
/// to _ptrcall_, which passes direct pointers to Rust objects).
///
/// Allows to inspect the involved class and method via `class_name()` and `method_name()`. Implements the `std::error::Error` trait, so
/// it comes with `Display` and `Error::source()` APIs.
///
/// # Possible error causes
/// Several reasons can cause a function call to fail. The reason is described in the `Display` impl.
///
/// - **Invalid method**: The method does not exist on the object.
/// - **Failed argument conversion**: The arguments passed to the method cannot be converted to the declared parameter types.
/// - **Failed return value conversion**: The returned `Variant` of a dynamic method cannot be converted to the expected return type.
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
/// # use godot_core::meta::error::{CallError, ConvertError};
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
///     let mut obj: Gd<MyClass> = MyClass::new_gd();
///
///     // Dynamic call. Note: forgot to pass the argument.
///     let result: Result<Variant, CallError> = obj.try_call("my_method", &[]);
///
///     // Get immediate and original errors.
///     // Note that source() can be None or of type ConvertError.
///     let outer: CallError = result.unwrap_err();
///     let inner: &CallError = outer.source().unwrap().downcast_ref::<CallError>().unwrap();
///
///     // Immediate error: try_call() internally invokes Object::call().
///     assert_eq!(outer.class_name(), Some("Object"));
///     assert_eq!(outer.method_name(), "call");
///
///     // Original error: my_method() failed.
///     assert_eq!(inner.class_name(), Some("MyClass"));
///     assert_eq!(inner.method_name(), "my_method");
/// }
pub struct CallError {
    // Boxed since the original struct is >= 176 bytes, making Result<..., CallError> very large.
    b: Box<InnerCallError>,
}

/// Inner struct. All functionality on outer `impl`.
#[derive(Debug)]
struct InnerCallError {
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
        if self.b.class_name.is_empty() {
            None
        } else {
            Some(&self.b.class_name)
        }
    }

    /// Name of the function or method that failed.
    pub fn method_name(&self) -> &str {
        &self.b.function_name
    }

    // ------------------------------------------------------------------------------------------------------------------------------------------
    // Constructors returning Result<(), Self>; possible failure

    /// Checks whether number of arguments matches the number of parameters.
    pub(crate) fn check_arg_count(
        call_ctx: &CallContext,
        arg_count: usize,
        param_count: usize,
    ) -> Result<(), Self> {
        // This will need to be adjusted once optional parameters are supported in #[func].
        if arg_count == param_count {
            return Ok(());
        }

        let call_error = Self::failed_param_count(call_ctx, arg_count, param_count);

        Err(call_error)
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
            format!(", [va] {}", join_args(varargs.iter().cloned()))
        };

        let call_expr = format!("{call_ctx}({explicit_args_str}{vararg_str})");

        // If the call error encodes an error generated by us, decode it.
        let mut source_error = None;
        if err.error == sys::GODOT_RUST_CUSTOM_CALL_ERROR {
            source_error = crate::private::call_error_remove(&err).map(SourceError::Call);
        }

        Err(Self::failed_varcall_inner(
            call_ctx,
            call_expr,
            err,
            &arg_types,
            explicit_args.len(),
            source_error,
        ))
    }

    // ------------------------------------------------------------------------------------------------------------------------------------------
    // Constructors returning Self; guaranteed failure

    /// Returns an error for a failed parameter conversion.
    pub(crate) fn failed_param_conversion<P>(
        call_ctx: &CallContext,
        param_index: isize,
        convert_error: ConvertError,
    ) -> Self {
        let param_ty = std::any::type_name::<P>();

        Self::new(
            call_ctx,
            format!("parameter #{param_index} ({param_ty}) conversion"),
            Some(convert_error),
        )
    }

    fn failed_param_conversion_engine(
        call_ctx: &CallContext,
        param_index: i32,
        actual: VariantType,
        expected: VariantType,
    ) -> Self {
        // Note: reason is same wording as in FromVariantError::description().
        let reason =
            format!("parameter #{param_index} -- cannot convert from {actual:?} to {expected:?}");

        Self::new(call_ctx, reason, None)
    }

    /// Returns an error for a failed return type conversion.
    ///
    /// **Note:** There are probably no practical scenarios where this occurs. Different calls:
    /// - outbound engine API: return values are statically typed (correct by binding) or Variant (infallible)
    /// - #[func] methods: dynamic calls return Variant
    /// - GDScript -> Rust calls: value is checked on GDScript side (at parse or runtime), not involving this.
    ///
    /// It might only occur if there are mistakes in the binding, or if we at some point add typed dynamic calls, à la `call<R>((1, "str"))`.
    pub(crate) fn failed_return_conversion<R>(
        call_ctx: &CallContext,
        convert_error: ConvertError,
    ) -> Self {
        let return_ty = std::any::type_name::<R>();

        Self::new(
            call_ctx,
            format!("return value {return_ty} conversion"),
            Some(convert_error),
        )
    }

    fn failed_param_count(
        call_ctx: &CallContext,
        arg_count: usize,
        param_count: usize,
    ) -> CallError {
        let param_plural = plural(param_count);
        let arg_plural = plural(arg_count);

        Self::new(
            call_ctx,
            format!(
                "function has {param_count} parameter{param_plural}, but received {arg_count} argument{arg_plural}"
            ),
            None,
        )
    }

    fn failed_varcall_inner(
        call_ctx: &CallContext,
        call_expr: String,
        err: sys::GDExtensionCallError,
        arg_types: &[VariantType],
        vararg_offset: usize,
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

        let mut call_error = match error {
            sys::GDEXTENSION_CALL_ERROR_INVALID_METHOD => {
                Self::new(call_ctx, "method not found", None)
            }
            sys::GDEXTENSION_CALL_ERROR_INVALID_ARGUMENT => {
                // Index calculation relies on patterns like call("...", varargs), might not always work...
                let from = arg_types[vararg_offset + argument as usize];
                let to = VariantType::from_sys(expected as sys::GDExtensionVariantType);
                let i = argument + 1;

                Self::failed_param_conversion_engine(call_ctx, i, from, to)
            }
            sys::GDEXTENSION_CALL_ERROR_TOO_MANY_ARGUMENTS
            | sys::GDEXTENSION_CALL_ERROR_TOO_FEW_ARGUMENTS => {
                let arg_count = arg_types.len() - vararg_offset;
                let param_count = expected as usize;
                Self::failed_param_count(call_ctx, arg_count, param_count)
            }
            sys::GDEXTENSION_CALL_ERROR_INSTANCE_IS_NULL => {
                Self::new(call_ctx, "instance is null", None)
            }
            sys::GDEXTENSION_CALL_ERROR_METHOD_NOT_CONST => {
                Self::new(call_ctx, "method is not const", None)
            }
            sys::GODOT_RUST_CUSTOM_CALL_ERROR => {
                // Not emitted by Godot.
                Self::new(call_ctx, String::new(), None)
            }
            _ => Self::new(
                call_ctx,
                format!("unknown reason (error code {error})"),
                None,
            ),
        };

        // Self {
        //     class_name: call_ctx.class_name.to_string(),
        //     function_name: call_ctx.function_name.to_string(),
        //     call_expr,
        //     reason,
        //     source,
        // }

        call_error.b.source = source;
        call_error.b.call_expr = call_expr;
        call_error
    }

    #[doc(hidden)]
    pub fn failed_by_user_panic(call_ctx: &CallContext, panic_payload: PanicPayload) -> Self {
        // This can cause the panic message to be printed twice in some scenarios (e.g. bind_mut() borrow failure).
        // But in other cases (e.g. itest `dynamic_call_with_panic`), it is only printed once.
        // Would need some work to have a consistent experience.

        let reason = panic_payload.into_panic_message();

        Self::new(call_ctx, format!("function panicked: {reason}"), None)
    }

    fn new(
        call_ctx: &CallContext,
        reason: impl Into<String>,
        source: Option<ConvertError>,
    ) -> Self {
        let inner = InnerCallError {
            class_name: call_ctx.class_name.to_string(),
            function_name: call_ctx.function_name.to_string(),
            call_expr: format!("{call_ctx}()"),
            reason: reason.into(),
            source: source.map(|e| SourceError::Convert {
                value: e.value().map_or_else(String::new, |v| format!("{v:?}")),
                erased_error: e.into(),
            }),
        };

        Self { b: Box::new(inner) }
    }

    /// Describes the error.
    ///
    /// This is the same as the `Display`/`ToString` repr, but without the prefix mentioning that this is a function call error,
    /// and without any source error information.
    pub fn message(&self, with_source: bool) -> String {
        let InnerCallError {
            call_expr,
            reason,
            source,
            ..
        } = &*self.b;

        let reason_str = if reason.is_empty() {
            String::new()
        } else {
            format!("\n    Reason: {reason}")
        };

        // let source_str = if with_source {
        //     self.source()
        //         .map(|e| format!("\n  Source: {}", e))
        //         .unwrap_or_default()
        // } else {
        //     String::new()
        // };

        let source_str = match source {
            Some(SourceError::Convert {
                erased_error,
                value,
            }) if with_source => {
                format!(
                    "\n  Source: {erased_error}{}{value}",
                    if value.is_empty() { "" } else { ": " },
                )
            }
            Some(SourceError::Call(e)) if with_source => {
                let message = e.message(true);
                format!("\n  Source: {message}")
            }
            _ => String::new(),
        };

        format!("{call_expr}{reason_str}{source_str}")
    }
}

impl fmt::Display for CallError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = self.message(true);
        write!(f, "godot-rust function call failed: {message}")
    }
}

impl fmt::Debug for CallError {
    // Delegate to inner box.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.b)
    }
}

impl Error for CallError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self.b.source.as_ref() {
            Some(SourceError::Convert {
                erased_error: e, ..
            }) => deref_to::<ErasedConvertError>(e),
            Some(SourceError::Call(e)) => deref_to::<CallError>(e),
            None => None,
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Implementation

#[derive(Debug)]
enum SourceError {
    Convert {
        erased_error: ErasedConvertError,
        value: String,
    },

    // If the top-level Box on CallError is ever removed, this would need to store Box<CallError> again.
    Call(CallError),
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

fn plural(count: usize) -> &'static str {
    if count == 1 {
        ""
    } else {
        "s"
    }
}
