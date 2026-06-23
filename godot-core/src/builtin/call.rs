/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Unified call builder, shared by `Gd`, `Variant` and `Callable`.

use std::borrow::Cow;
use std::future::Future;

use godot_ffi as sys;

use crate::builtin::{AnyArray, Callable, StringName, Variant, VariantType};
use crate::meta::error::CallError;
use crate::meta::{CallContext, resolve_gdscript_coroutine};

/// Builder for an untyped (varcall) method call.
///
/// Created by `call_ex()` on [`Gd`][crate::obj::Gd], [`Variant`] and [`Callable`]. Arguments are supplied with [`args()`][Self::args] or
/// [`args_array()`][Self::args_array] (omit both for a no-argument call); a terminal operation then selects timing and result type:
///
/// | Terminal                         | Result                          | Replaces                              |
/// |----------------------------------|---------------------------------|---------------------------------------|
/// | [`done()`][Self::done]           | `Variant`                       | `call` / `callv`                      |
/// | [`try_done()`][Self::try_done]   | `Result<Variant, CallError>`    | `Object::try_call`                    |
/// | [`deferred()`][Self::deferred]   | `()` (runs at idle time)        | `Object::call_deferred`               |
/// | [`to_future()`][Self::to_future] | `impl Future<Output = Variant>` | awaiting GDScript coroutines          |
///
/// The bare `call()` shorthand is equivalent to `call_ex(method).args(args).done()`.
#[must_use]
pub struct ExCall<'ex> {
    target: CallTarget<'ex>,
    // Empty when `target` is a `Callable` invocation (no method name).
    method: StringName,
    args: CallArgs<'ex>,
}

enum CallTarget<'ex> {
    /// Method call on a `Variant` receiver (objects and builtins). Owned for `Gd`, borrowed for `Variant`.
    Method(Cow<'ex, Variant>),
    /// Direct `Callable` invocation; there is no method name.
    Invoke(Cow<'ex, Callable>),
}

enum CallArgs<'ex> {
    Slice(&'ex [Variant]),
    Array(&'ex AnyArray),
}

impl<'ex> ExCall<'ex> {
    pub(crate) fn on_variant(variant: &'ex Variant, method: StringName) -> Self {
        Self {
            target: CallTarget::Method(Cow::Borrowed(variant)),
            method,
            args: CallArgs::Slice(&[]),
        }
    }

    pub(crate) fn on_owned_variant(variant: Variant, method: StringName) -> Self {
        Self {
            target: CallTarget::Method(Cow::Owned(variant)),
            method,
            args: CallArgs::Slice(&[]),
        }
    }

    pub(crate) fn on_callable(callable: &'ex Callable) -> Self {
        Self {
            target: CallTarget::Invoke(Cow::Borrowed(callable)),
            method: StringName::default(),
            args: CallArgs::Slice(&[]),
        }
    }

    // ------------------------------------------------------------------------------------------------------------------------------------------
    // Modifiers

    /// Pass the arguments as a slice of [`Variant`]s.
    ///
    /// Replaces any previously set arguments. Equivalent to the second parameter of the old `call()` methods.
    pub fn args(mut self, args: &'ex [Variant]) -> Self {
        self.args = CallArgs::Slice(args);
        self
    }

    /// Pass the arguments as a dynamic [`AnyArray`] instead of a slice.
    ///
    /// Replaces any previously set arguments. Equivalent to the old `callv()` methods.
    pub fn args_array(mut self, array: &'ex AnyArray) -> Self {
        self.args = CallArgs::Array(array);
        self
    }

    // ------------------------------------------------------------------------------------------------------------------------------------------
    // Terminal operations

    /// Immediate synchronous call. Returns the method's return value.
    ///
    /// # Panics
    /// If the method does not exist, the arguments don't match, or the call errors. Use [`try_done()`][Self::try_done] for a fallible variant.
    pub fn done(self) -> Variant {
        self.with_slice(|target, method, args| match target {
            CallTarget::Method(v) => v.call_inner(method, args),
            CallTarget::Invoke(c) => c.call(args),
        })
    }

    /// Fallible variant of [`done()`][Self::done], surfacing call errors instead of panicking.
    ///
    /// Note: a `Callable` invocation cannot surface call errors (Godot returns `NIL` on failure), so it always reports `Ok`.
    pub fn try_done(self) -> Result<Variant, CallError> {
        self.with_slice(|target, method, args| match target {
            CallTarget::Method(v) => v.try_call_inner(method, args),
            CallTarget::Invoke(c) => Ok(c.call(args)),
        })
    }

    /// Runs the call at idle time instead of immediately. The return value is not available, hence `()`.
    ///
    /// Only valid on an object receiver: deferred dispatch goes through a [`Callable`], which Godot can only bind to objects; it is also
    /// meaningless on a builtin, since that is a value copy the deferred call could not mutate in place. Calls via [`Gd`][crate::obj::Gd] or
    /// [`Callable`] always have an object target. On a builtin `Variant` receiver (e.g. `String`, `Array`) the call is a no-op; in strict mode
    /// (default in debug builds) this panics instead.
    pub fn deferred(self) {
        self.with_slice(|target, method, args| match target {
            CallTarget::Method(v) => {
                sys::strict_assert_eq!(
                    v.get_type(),
                    VariantType::OBJECT,
                    "call_ex(...).deferred() requires an object receiver, but got a builtin Variant of type {:?}",
                    v.get_type()
                );
                let callable = Callable::from_variant_method(v, method);
                callable.call_deferred(args);
            }
            CallTarget::Invoke(c) => c.call_deferred(args),
        });
    }

    /// Turns the call into a future, awaiting GDScript coroutines (`await`) before resolving.
    ///
    /// Performs the synchronous call immediately (like [`done()`][Self::done]); the returned future only keeps the coroutine handle alive. If the
    /// method does not use `await`, the future resolves immediately to its return value.
    ///
    /// # Panics
    /// Same as [`done()`][Self::done].
    pub fn to_future(self) -> impl Future<Output = Variant> + 'static {
        let variant = self.done();

        // For `Ret = Variant`, the conversion in `resolve_gdscript_coroutine` is the identity and never fails, so the context is only nominal.
        let call_ctx = CallContext::outbound("Variant", "call");
        resolve_gdscript_coroutine::<Variant>(call_ctx, variant)
    }

    // ------------------------------------------------------------------------------------------------------------------------------------------
    // Internal

    /// Materializes the arguments as a slice (converting from `AnyArray` if necessary) and dispatches on the target.
    fn with_slice<R>(&self, f: impl FnOnce(&CallTarget<'ex>, &StringName, &[Variant]) -> R) -> R {
        match &self.args {
            CallArgs::Slice(slice) => f(&self.target, &self.method, slice),
            CallArgs::Array(array) => {
                let vec: Vec<Variant> = array.iter_shared().collect();
                f(&self.target, &self.method, &vec)
            }
        }
    }
}
