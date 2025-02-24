/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot_ffi as sys;

use crate::builtin::{inner, GString, StringName, Variant, VariantArray};
use crate::classes;
use crate::meta::{GodotType, ToGodot};
use crate::obj::bounds::DynMemory;
use crate::obj::Bounds;
use crate::obj::{Gd, GodotClass, InstanceId};
use std::{fmt, ptr};
use sys::{ffi_methods, GodotFfi};

#[cfg(all(since_api = "4.2", before_api = "4.3"))] #[cfg_attr(published_docs, doc(cfg(all(since_api = "4.2", before_api = "4.3"))))]
type CallableCustomInfo = sys::GDExtensionCallableCustomInfo;
#[cfg(since_api = "4.3")] #[cfg_attr(published_docs, doc(cfg(since_api = "4.3")))]
type CallableCustomInfo = sys::GDExtensionCallableCustomInfo2;

/// A `Callable` represents a function in Godot.
///
/// Callables can be created in many ways:
/// - From an `Object` and a (non-static) method name. This is a _standard_ callable.
/// - From a GDScript class name and a static function name. (This typically works because classes are instances of `GDScript`).
/// - From a GDScript lambda function.
/// - By modifying an existing `Callable` with [`bind()`][Self::bind] or [`unbind()`][Self::unbind].
/// - By creating a custom callable from Rust.
///
/// # Godot docs
///
/// [`Callable` (stable)](https://docs.godotengine.org/en/stable/classes/class_callable.html)
pub struct Callable {
    opaque: sys::types::OpaqueCallable,
}

impl Callable {
    fn from_opaque(opaque: sys::types::OpaqueCallable) -> Self {
        Self { opaque }
    }

    /// Create a callable for the non-static method `object.method_name`.
    ///
    /// See also [`Gd::callable()`].
    ///
    /// _Godot equivalent: `Callable(Object object, StringName method)`_
    pub fn from_object_method<T, S>(object: &Gd<T>, method_name: S) -> Self
    where
        T: GodotClass, // + Inherits<Object>,
        S: meta::AsArg<StringName>,
    {
        meta::arg_into_ref!(method_name);

        unsafe {
            Self::new_with_uninit(|self_ptr| {
                let ctor = sys::builtin_fn!(callable_from_object_method);
                let raw = object.to_ffi();
                let args = [raw.as_arg_ptr(), method_name.sys()];
                ctor(self_ptr, args.as_ptr());
            })
        }
    }

    /// Create a callable for the static method `class_name::function` (single-threaded).
    ///
    /// Allows you to call static functions through `Callable`.
    ///
    /// Note that due to varying support across different engine versions, the resulting `Callable` has unspecified behavior for
    /// methods such as [`method_name()`][Self::method_name], [`object()`][Self::object], [`object_id()`][Self::object_id] or
    /// [`get_argument_count()`][Self::arg_len] among others. It is recommended to only use this for calling the function.
    ///
    /// # Compatibility
    /// Up until and including Godot 4.3, this method has some limitations:
    /// - [`is_valid()`][Self::is_valid] will return `false`, even though the call itself succeeds.
    /// - You cannot use statics to connect signals to such callables. Use the new typed signal API instead.
    pub fn from_local_static(
        class_name: impl meta::AsArg<StringName>,
        function_name: impl meta::AsArg<StringName>,
    ) -> Self {
        meta::arg_into_owned!(class_name);
        meta::arg_into_owned!(function_name);

        // Modern implementation: use ClassDb::class_call_static().
        #[cfg(since_api = "4.4")]
        {
            let callable_name = format!("{class_name}.{function_name}");

            Self::from_local_fn(&callable_name, move |args| {
                let args = args.iter().cloned().cloned().collect::<Vec<_>>();

                let result: Variant = classes::ClassDb::singleton().class_call_static(
                    &class_name,
                    &function_name,
                    args.as_slice(),
                );
                Ok(result)
            })
        }

        // Polyfill for <= Godot 4.3: use GDScript expressions.
        #[cfg(before_api = "4.4")]
        {
            use crate::obj::NewGd;

            let code = format!(
                "static func __callable():\n\treturn Callable({class_name}, \"{function_name}\")"
            );

            let mut script = classes::GDScript::new_gd();
            script.set_source_code(&code);
            script.reload();

            let callable = script.call("__callable", &[]);
            callable.to()
        }
    }

    #[cfg(since_api = "4.2")] #[cfg_attr(published_docs, doc(cfg(since_api = "4.2")))]
    fn default_callable_custom_info() -> CallableCustomInfo {
        CallableCustomInfo {
            callable_userdata: ptr::null_mut(),
            token: ptr::null_mut(),
            object_id: 0,
            call_func: None,
            is_valid_func: None, // could be customized, but no real use case yet.
            free_func: None,
            hash_func: None,
            equal_func: None,
            // Op < is only used in niche scenarios and default is usually good enough, see https://github.com/godotengine/godot/issues/81901.
            less_than_func: None,
            to_string_func: None,
            #[cfg(since_api = "4.3")] #[cfg_attr(published_docs, doc(cfg(since_api = "4.3")))]
            get_argument_count_func: None,
        }
    }

    /// Create callable from **single-threaded** Rust function or closure.
    ///
    /// `name` is used for the string representation of the closure, which helps debugging.
    ///
    /// This constructor only allows the callable to be invoked from the same thread as creating it. If you need to invoke it from any thread,
    /// use [`from_sync_fn`][Self::from_sync_fn] instead (requires crate feature `experimental-threads`; only enable if really needed).
    #[cfg(since_api = "4.2")] #[cfg_attr(published_docs, doc(cfg(since_api = "4.2")))]
    pub fn from_local_fn<F, S>(name: S, rust_function: F) -> Self
    where
        F: 'static + FnMut(&[&Variant]) -> Result<Variant, ()>,
        S: meta::AsArg<GString>,
    {
        meta::arg_into_owned!(name);

        Self::from_fn_wrapper(FnWrapper {
            rust_function,
            name,
            thread_id: Some(std::thread::current().id()),
        })
    }

    /// Create callable from **thread-safe** Rust function or closure.
    ///
    /// `name` is used for the string representation of the closure, which helps debugging.
    ///
    /// This constructor requires `Send` + `Sync` bound and allows the callable to be invoked from any thread. If you guarantee that you invoke
    /// it from the same thread as creating it, use [`from_local_fn`][Self::from_local_fn] instead.
    ///
    /// Callables created through multiple `from_local_fn` or `from_sync_fn()` calls are never equal, even if they refer to the same function.
    /// If you want to use equality, either clone an existing `Callable` instance, or define your own `PartialEq` impl with
    /// [`Callable::from_custom`].
    ///
    /// # Example
    /// ```no_run
    /// # use godot::prelude::*;
    /// let callable = Callable::from_sync_fn("sum", |args: &[&Variant]| {
    ///     let sum: i32 = args.iter().map(|arg| arg.to::<i32>()).sum();
    ///     Ok(sum.to_variant())
    /// });
    /// ```
    #[cfg(since_api = "4.2")] #[cfg_attr(published_docs, doc(cfg(since_api = "4.2")))]
    #[cfg(feature = "experimental-threads")]
    pub fn from_sync_fn<F, S>(name: S, rust_function: F) -> Self
    where
        F: 'static + Send + Sync + FnMut(&[&Variant]) -> Result<Variant, ()>,
        S: meta::AsArg<GString>,
    {
        meta::arg_into_owned!(name);

        Self::from_fn_wrapper(FnWrapper {
            rust_function,
            name,
            thread_id: None,
        })
    }

    #[deprecated = "Now split into from_local_fn (single-threaded) and from_sync_fn (multi-threaded)."]
    #[cfg(since_api = "4.2")] #[cfg_attr(published_docs, doc(cfg(since_api = "4.2")))]
    pub fn from_fn<F, S>(name: S, rust_function: F) -> Self
    where
        F: 'static + Send + Sync + FnMut(&[&Variant]) -> Result<Variant, ()>,
        S: Into<GString>,
    {
        // Do not call from_sync_fn() since that is feature-gated, but this isn't due to compatibility.
        Self::from_fn_wrapper(FnWrapper {
            rust_function,
            name: name.into(),
            thread_id: None,
        })
    }

    /// Create a highly configurable callable from Rust.
    ///
    /// See [`RustCallable`] for requirements on the type.
    #[cfg(since_api = "4.2")] #[cfg_attr(published_docs, doc(cfg(since_api = "4.2")))]
    pub fn from_custom<C: RustCallable>(callable: C) -> Self {
        // Could theoretically use `dyn` but would need:
        // - double boxing
        // - a type-erased workaround for PartialEq supertrait (which has a `Self` type parameter and thus is not object-safe)
        let userdata = CallableUserdata { inner: callable };

        let info = CallableCustomInfo {
            callable_userdata: Box::into_raw(Box::new(userdata)) as *mut std::ffi::c_void,
            call_func: Some(rust_callable_call_custom::<C>),
            free_func: Some(rust_callable_destroy::<C>),
            hash_func: Some(rust_callable_hash::<C>),
            equal_func: Some(rust_callable_equal::<C>),
            to_string_func: Some(rust_callable_to_string_display::<C>),
            ..Self::default_callable_custom_info()
        };

        Self::from_custom_info(info)
    }

    #[cfg(since_api = "4.2")] #[cfg_attr(published_docs, doc(cfg(since_api = "4.2")))]
    fn from_fn_wrapper<F>(inner: FnWrapper<F>) -> Self
    where
        F: FnMut(&[&Variant]) -> Result<Variant, ()>,
    {
        let userdata = CallableUserdata { inner };

        let info = CallableCustomInfo {
            callable_userdata: Box::into_raw(Box::new(userdata)) as *mut std::ffi::c_void,
            call_func: Some(rust_callable_call_fn::<F>),
            free_func: Some(rust_callable_destroy::<FnWrapper<F>>),
            to_string_func: Some(rust_callable_to_string_named::<F>),
            ..Self::default_callable_custom_info()
        };

        Self::from_custom_info(info)
    }

    #[cfg(since_api = "4.2")] #[cfg_attr(published_docs, doc(cfg(since_api = "4.2")))]
    fn from_custom_info(mut info: CallableCustomInfo) -> Callable {
        // SAFETY: callable_custom_create() is a valid way of creating callables.
        unsafe {
            Callable::new_with_uninit(|type_ptr| {
                #[cfg(before_api = "4.3")]
                {
                    sys::interface_fn!(callable_custom_create)(type_ptr, ptr::addr_of_mut!(info))
                }
                #[cfg(since_api = "4.3")]
                {
                    sys::interface_fn!(callable_custom_create2)(type_ptr, ptr::addr_of_mut!(info))
                }
            })
        }
    }

    /// Creates an invalid/empty object that cannot be called.
    ///
    /// _Godot equivalent: `Callable()`_
    pub fn invalid() -> Self {
        unsafe {
            Self::new_with_uninit(|self_ptr| {
                let ctor = sys::builtin_fn!(callable_construct_default);
                ctor(self_ptr, ptr::null_mut())
            })
        }
    }

    /// Calls the method represented by this callable.
    ///
    /// Arguments passed should match the method's signature.
    ///
    /// - If called with more arguments than expected by the method, the extra arguments will be ignored and
    ///   the call continues as normal.
    /// - If called with fewer arguments than expected it will crash Godot, without triggering UB.
    /// - If called with arguments of the wrong type then an error will be printed and the call will return
    ///   `NIL`.
    /// - If called on an invalid Callable then no error is printed, and `NIL` is returned.
    ///
    /// _Godot equivalent: `callv`_
    pub fn callv(&self, arguments: &VariantArray) -> Variant {
        self.as_inner().callv(arguments)
    }

    /// Returns a copy of this Callable with one or more arguments bound, reading them from an array.
    ///
    /// _Godot equivalent: `bindv`_
    pub fn bindv(&self, arguments: &VariantArray) -> Self {
        self.as_inner().bindv(arguments)
    }

    /// Returns the name of the method represented by this callable. If the callable is a lambda function,
    /// returns the surrounding function's name.
    ///
    /// ## Known Bugs
    ///
    /// Getting the name of a lambda errors instead of returning its name, see [godot#73052].
    ///
    /// _Godot equivalent: `get_method`_
    ///
    /// [godot#73052]: https://github.com/godotengine/godot/issues/73052
    #[doc(alias = "get_method")]
    pub fn method_name(&self) -> Option<StringName> {
        let method_name = self.as_inner().get_method();
        if method_name.is_empty() {
            None
        } else {
            Some(method_name)
        }
    }

    /// Returns the object on which this callable is called.
    ///
    /// Returns `None` when this callable doesn't have any target object to call a method on (regardless of whether the method exists for that
    /// target or not). Also returns `None` if the object is dead. You can differentiate these two cases using [`object_id()`][Self::object_id].
    ///
    /// _Godot equivalent: `get_object`_
    pub fn object(&self) -> Option<Gd<classes::Object>> {
        // Increment refcount because we're getting a reference, and `InnerCallable::get_object` doesn't
        // increment the refcount.
        self.as_inner().get_object().map(|mut object| {
            <classes::Object as Bounds>::DynMemory::maybe_inc_ref(&mut object.raw);
            object
        })
    }

    /// Returns the ID of this callable's object, see also [`Gd::instance_id`].
    ///
    /// Returns `None` when this callable doesn't have any target to call a method on.
    ///
    /// If the pointed-to object is dead, the ID will still be returned. Use [`object()`][Self::object] to check for liveness.
    ///
    /// _Godot equivalent: `get_object_id`_
    pub fn object_id(&self) -> Option<InstanceId> {
        let id = self.as_inner().get_object_id();
        InstanceId::try_from_i64(id)
    }

    /// Returns the 32-bit hash value of this callable's object.
    ///
    /// _Godot equivalent: `hash`_
    pub fn hash(&self) -> u32 {
        self.as_inner().hash().try_into().unwrap()
    }

    /// Returns true if this callable is a custom callable.
    ///
    /// Custom callables are mainly created from bind or unbind. In GDScript, lambda functions are also
    /// custom callables.
    ///
    /// If a callable is not a custom callable, then it is considered a standard callable, this function is
    /// the opposite of [`Callable.is_standard`].
    ///
    /// _Godot equivalent: `is_custom`_
    ///
    /// [`Callable.is_standard`]: https://docs.godotengine.org/en/stable/classes/class_callable.html#class-callable-method-is-standard
    #[doc(alias = "is_standard")]
    pub fn is_custom(&self) -> bool {
        self.as_inner().is_custom()
    }

    /// Returns true if this callable has no target to call the method on.
    ///
    /// This is not the negated form of [`is_valid`][Self::is_valid], as `is_valid` will return `false` if the callable has a
    /// target but the method does not exist.
    ///
    /// _Godot equivalent: `is_null`_
    pub fn is_null(&self) -> bool {
        self.as_inner().is_null()
    }

    /// Returns true if the callable's object exists and has a valid method name assigned, or is a custom
    /// callable.
    ///
    /// _Godot equivalent: `is_valid`_
    pub fn is_valid(&self) -> bool {
        self.as_inner().is_valid()
    }

    /// Returns a copy of the callable, ignoring `args` user arguments.
    ///
    /// Despite its name, this does **not** directly undo previous `bind()` calls. See
    /// [Godot docs](https://docs.godotengine.org/en/latest/classes/class_callable.html#class-callable-method-unbind) for up-to-date semantics.
    pub fn unbind(&self, args: usize) -> Callable {
        self.as_inner().unbind(args as i64)
    }

    #[cfg(since_api = "4.3")] #[cfg_attr(published_docs, doc(cfg(since_api = "4.3")))]
    pub fn get_argument_count(&self) -> usize {
        self.as_inner().get_argument_count() as usize
    }

    /// Get number of bound arguments.
    ///
    /// Note: for Godot < 4.4, this function returns incorrect results when applied on a callable that used `unbind()`.
    /// See [#98713](https://github.com/godotengine/godot/pull/98713) for details.
    pub fn get_bound_arguments_count(&self) -> usize {
        // This does NOT fix the bug before Godot 4.4, just cap it at zero. unbind() will still erroneously decrease the bound arguments count.
        let alleged_count = self.as_inner().get_bound_arguments_count();

        alleged_count.max(0) as usize
    }

    #[doc(hidden)]
    pub fn as_inner(&self) -> inner::InnerCallable {
        inner::InnerCallable::from_outer(self)
    }
}

impl_builtin_traits! {
    for Callable {
        // Default is absent by design, to encourage explicit valid initialization.

        Clone => callable_construct_copy;
        Drop => callable_destroy;

        // Equality for custom callables depend on the equality implementation of that custom callable.
        // So we cannot implement `Eq` here and be confident equality will be total for all future custom callables.
        PartialEq => callable_operator_equal;
        // Godot does not define a less-than operator, so there is no `PartialOrd`.
        // Hash could be added, but without Eq it's not that useful; wait for actual use cases.
    }
}

// SAFETY:
// The `opaque` in `Callable` is just a pair of pointers, and requires no special initialization or cleanup
// beyond what is done in `from_opaque` and `drop`. So using `*mut Opaque` is safe.
unsafe impl GodotFfi for Callable {
    fn variant_type() -> sys::VariantType {
        sys::VariantType::CALLABLE
    }

    ffi_methods! { type sys::GDExtensionTypePtr = *mut Opaque;
        fn new_from_sys;
        fn new_with_uninit;
        fn from_arg_ptr;
        fn sys;
        fn sys_mut;
        fn move_return_ptr;
    }

    unsafe fn new_with_init(init_fn: impl FnOnce(sys::GDExtensionTypePtr)) -> Self {
        let mut result = Self::invalid();
        init_fn(result.sys_mut());
        result
    }
}

meta::impl_godot_as_self!(Callable);

impl fmt::Debug for Callable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let method = self.method_name();
        let object = self.object();

        f.debug_struct("Callable")
            .field("method", &method)
            .field("object", &object)
            .finish()
    }
}

impl fmt::Display for Callable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_variant())
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Callbacks for custom implementations

#[cfg(since_api = "4.2")] #[cfg_attr(published_docs, doc(cfg(since_api = "4.2")))]
use custom_callable::*;

use crate::meta;
#[cfg(since_api = "4.2")] #[cfg_attr(published_docs, doc(cfg(since_api = "4.2")))]
pub use custom_callable::RustCallable;

#[cfg(since_api = "4.2")] #[cfg_attr(published_docs, doc(cfg(since_api = "4.2")))]
mod custom_callable {
    use super::*;
    use crate::builtin::GString;
    use std::hash::Hash;
    use std::thread::ThreadId;

    pub struct CallableUserdata<T> {
        pub inner: T,
    }

    impl<T> CallableUserdata<T> {
        /// # Safety
        /// Returns an unbounded reference. `void_ptr` must be a valid pointer to a `CallableUserdata`.
        unsafe fn inner_from_raw<'a>(void_ptr: *mut std::ffi::c_void) -> &'a mut T {
            let ptr = void_ptr as *mut CallableUserdata<T>;
            &mut (*ptr).inner
        }
    }

    pub(crate) struct FnWrapper<F> {
        pub(super) rust_function: F,
        pub(super) name: GString,

        /// `None` if the callable is multi-threaded ([`Callable::from_sync_fn`]).
        pub(super) thread_id: Option<ThreadId>,
    }

    /// Represents a custom callable object defined in Rust.
    ///
    /// This trait has a single method, `invoke`, which is called upon invocation.
    ///
    /// Since callables can be invoked from anywhere, they must be self-contained (`'static`) and thread-safe (`Send + Sync`).
    /// They also should implement `Display` for the Godot string representation.
    /// Furthermore, `PartialEq` and `Hash` are required for equality checks and usage as a key in a `Dictionary`.
    pub trait RustCallable: 'static + PartialEq + Hash + fmt::Display + Send + Sync {
        /// Invokes the callable with the given arguments as `Variant` references.
        ///
        /// Return `Ok(...)` if the call succeeded, and `Err(())` otherwise.
        /// Error handling is mostly needed in case argument number or types mismatch.
        #[allow(clippy::result_unit_err)] // TODO remove once there's a clear error type here.
        fn invoke(&mut self, args: &[&Variant]) -> Result<Variant, ()>;
    }

    pub unsafe extern "C" fn rust_callable_call_custom<C: RustCallable>(
        callable_userdata: *mut std::ffi::c_void,
        p_args: *const sys::GDExtensionConstVariantPtr,
        p_argument_count: sys::GDExtensionInt,
        r_return: sys::GDExtensionVariantPtr,
        r_error: *mut sys::GDExtensionCallError,
    ) {
        let arg_refs: &[&Variant] = Variant::borrow_ref_slice(p_args, p_argument_count as usize);

        let name = {
            let c: &C = CallableUserdata::inner_from_raw(callable_userdata);
            c.to_string()
        };
        let ctx = meta::CallContext::custom_callable(name.as_str());

        crate::private::handle_varcall_panic(&ctx, &mut *r_error, move || {
            // Get the RustCallable again inside closure so it doesn't have to be UnwindSafe.
            let c: &mut C = CallableUserdata::inner_from_raw(callable_userdata);
            let result = c.invoke(arg_refs);
            meta::varcall_return_checked(result, r_return, r_error);
            Ok(())
        });
    }

    pub unsafe extern "C" fn rust_callable_call_fn<F>(
        callable_userdata: *mut std::ffi::c_void,
        p_args: *const sys::GDExtensionConstVariantPtr,
        p_argument_count: sys::GDExtensionInt,
        r_return: sys::GDExtensionVariantPtr,
        r_error: *mut sys::GDExtensionCallError,
    ) where
        F: FnMut(&[&Variant]) -> Result<Variant, ()>,
    {
        let arg_refs: &[&Variant] = Variant::borrow_ref_slice(p_args, p_argument_count as usize);

        let name = {
            let w: &FnWrapper<F> = CallableUserdata::inner_from_raw(callable_userdata);
            w.name.to_string()
        };
        let ctx = meta::CallContext::custom_callable(name.as_str());

        crate::private::handle_varcall_panic(&ctx, &mut *r_error, move || {
            // Get the FnWrapper again inside closure so the FnMut doesn't have to be UnwindSafe.
            let w: &mut FnWrapper<F> = CallableUserdata::inner_from_raw(callable_userdata);

            if w.thread_id
                .is_some_and(|tid| tid != std::thread::current().id())
            {
                // NOTE: this panic is currently not propagated to the caller, but results in an error message and Nil return.
                // See comments in itest callable_call() for details.
                panic!(
                    "Callable '{}' created with from_local_fn() must be called from the same thread it was created in.\n\
                    If you need to call it from any thread, use from_sync_fn() instead (requires `experimental-threads` feature).",
                    w.name
                );
            }

            let result = (w.rust_function)(arg_refs);
            meta::varcall_return_checked(result, r_return, r_error);
            Ok(())
        });
    }

    pub unsafe extern "C" fn rust_callable_destroy<T>(callable_userdata: *mut std::ffi::c_void) {
        let rust_ptr = callable_userdata as *mut CallableUserdata<T>;
        let _drop = Box::from_raw(rust_ptr);
    }

    pub unsafe extern "C" fn rust_callable_hash<T: Hash>(
        callable_userdata: *mut std::ffi::c_void,
    ) -> u32 {
        let c: &T = CallableUserdata::<T>::inner_from_raw(callable_userdata);

        // Just cut off top bits, not best-possible hash.
        sys::hash_value(c) as u32
    }

    pub unsafe extern "C" fn rust_callable_equal<T: PartialEq>(
        callable_userdata_a: *mut std::ffi::c_void,
        callable_userdata_b: *mut std::ffi::c_void,
    ) -> sys::GDExtensionBool {
        let a: &T = CallableUserdata::inner_from_raw(callable_userdata_a);
        let b: &T = CallableUserdata::inner_from_raw(callable_userdata_b);

        sys::conv::bool_to_sys(a == b)
    }

    pub unsafe extern "C" fn rust_callable_to_string_display<T: fmt::Display>(
        callable_userdata: *mut std::ffi::c_void,
        r_is_valid: *mut sys::GDExtensionBool,
        r_out: sys::GDExtensionStringPtr,
    ) {
        let c: &T = CallableUserdata::inner_from_raw(callable_userdata);
        let s = crate::builtin::GString::from(c.to_string());

        s.move_into_string_ptr(r_out);
        *r_is_valid = sys::conv::SYS_TRUE;
    }

    pub unsafe extern "C" fn rust_callable_to_string_named<F>(
        callable_userdata: *mut std::ffi::c_void,
        r_is_valid: *mut sys::GDExtensionBool,
        r_out: sys::GDExtensionStringPtr,
    ) {
        let w: &mut FnWrapper<F> = CallableUserdata::inner_from_raw(callable_userdata);

        w.name.clone().move_into_string_ptr(r_out);
        *r_is_valid = sys::conv::SYS_TRUE;
    }
}
