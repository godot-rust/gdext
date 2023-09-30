/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Functions and macros that are not very specific to gdext, but come in handy.

use crate as sys;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Macros

/// Verifies a condition at compile time.
// https://blog.rust-lang.org/2021/12/02/Rust-1.57.0.html#panic-in-const-contexts
#[macro_export]
macro_rules! static_assert {
    ($cond:expr) => {
        const _: () = assert!($cond);
    };
    ($cond:expr, $msg:literal) => {
        const _: () = assert!($cond, $msg);
    };
}

/// Verifies at compile time that two types `T` and `U` have the same size.
#[macro_export]
macro_rules! static_assert_eq_size {
    ($T:ty, $U:ty) => {
        godot_ffi::static_assert!(std::mem::size_of::<$T>() == std::mem::size_of::<$U>());
    };
    ($T:ty, $U:ty, $msg:literal) => {
        godot_ffi::static_assert!(std::mem::size_of::<$T>() == std::mem::size_of::<$U>(), $msg);
    };
}

/// Trace output.
#[cfg(feature = "trace")]
#[macro_export]
macro_rules! out {
    ()                          => (eprintln!());
    ($fmt:literal)              => (eprintln!($fmt));
    ($fmt:literal, $($arg:tt)*) => (eprintln!($fmt, $($arg)*));
}

/// Trace output.
#[cfg(not(feature = "trace"))]
// TODO find a better way than sink-writing to avoid warnings, #[allow(unused_variables)] doesn't work
#[macro_export]
macro_rules! out {
    ()                          => ({});
    ($fmt:literal)              => ({ use std::io::{sink, Write}; let _ = write!(sink(), $fmt); });
    ($fmt:literal, $($arg:tt)*) => ({ use std::io::{sink, Write}; let _ = write!(sink(), $fmt, $($arg)*); };)
}

/// Extract a function pointer from its `Option` and convert it to the (dereferenced) target type.
///
/// ```ignore
///  let get_godot_version = get_proc_address(sys::c_str(b"get_godot_version\0"));
///  let get_godot_version = sys::cast_fn_ptr!(get_godot_version as sys::GDExtensionInterfaceGetGodotVersion);
/// ```
#[allow(unused)]
#[macro_export]
macro_rules! cast_fn_ptr {
    ($option:ident as $ToType:ty) => {{
        let ptr = $option.expect("null function pointer");
        std::mem::transmute::<unsafe extern "C" fn(), <$ToType as $crate::Inner>::FnPtr>(ptr)
    }};
}

/// Makes sure that Godot is running, or panics. Debug mode only!
/// (private macro)
macro_rules! debug_assert_godot {
    ($expr:expr) => {
        debug_assert!(
            $expr,
            "Godot engine not available; make sure you are not calling it from unit/doc tests"
        );
    };
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Utility functions

/// Extract value from box before `into_inner()` is stable
#[allow(clippy::boxed_local)] // false positive
pub fn unbox<T>(value: Box<T>) -> T {
    // Deref-move is a Box magic feature; see https://stackoverflow.com/a/42264074
    *value
}

/// Explicitly cast away `const` from a pointer, similar to C++ `const_cast`.
///
/// The `as` conversion simultaneously doing 10 other things, potentially causing unintended transmutations.
pub fn force_mut_ptr<T>(ptr: *const T) -> *mut T {
    ptr as *mut T
}

/// Add `const` to a mut ptr.
pub fn to_const_ptr<T>(ptr: *mut T) -> *const T {
    ptr as *const T
}
/// If `ptr` is not null, returns `Some(mapper(ptr))`; otherwise `None`.
#[inline]
pub fn ptr_then<T, R, F>(ptr: *mut T, mapper: F) -> Option<R>
where
    F: FnOnce(*mut T) -> R,
{
    // Could also use NonNull in signature, but for this project we always deal with FFI raw pointers
    if ptr.is_null() {
        None
    } else {
        Some(mapper(ptr))
    }
}

/// Returns a C `const char*` for a null-terminated byte string.
#[inline]
pub fn c_str(s: &[u8]) -> *const std::ffi::c_char {
    // Ensure null-terminated
    debug_assert!(!s.is_empty() && s[s.len() - 1] == 0);

    s.as_ptr() as *const std::ffi::c_char
}

#[inline]
pub fn c_str_from_str(s: &str) -> *const std::ffi::c_char {
    debug_assert!(s.is_ascii());

    c_str(s.as_bytes())
}

/// Returns an ad-hoc hash of any object.
pub fn hash_value<T: std::hash::Hash>(t: &T) -> u64 {
    use std::hash::Hasher;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    t.hash(&mut hasher);
    hasher.finish()
}

/*
pub fn unqualified_type_name<T>() -> &'static str {
    let type_name = std::any::type_name::<T>();
    type_name.split("::").last().unwrap()
}
*/

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Private helpers

/// Metafunction to extract inner function pointer types from all the bindgen Option<F> type names.
/// Needed for `cast_fn_ptr` macro.
pub(crate) trait Inner: Sized {
    type FnPtr: Sized;

    fn extract(self, error_msg: &str) -> Self::FnPtr;
}

impl<T> Inner for Option<T> {
    type FnPtr = T;

    fn extract(self, error_msg: &str) -> Self::FnPtr {
        self.expect(error_msg)
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Function types used for table loaders

pub(crate) type GetClassMethod = unsafe extern "C" fn(
    p_classname: sys::GDExtensionConstStringNamePtr,
    p_methodname: sys::GDExtensionConstStringNamePtr,
    p_hash: sys::GDExtensionInt,
) -> sys::GDExtensionMethodBindPtr;

pub type ClassMethodBind = sys::GDExtensionMethodBindPtr;

pub(crate) type GetBuiltinMethod = unsafe extern "C" fn(
    p_type: sys::GDExtensionVariantType,
    p_method: sys::GDExtensionConstStringNamePtr,
    p_hash: sys::GDExtensionInt,
) -> sys::GDExtensionPtrBuiltInMethod;

// GDExtensionPtrBuiltInMethod
pub type BuiltinMethodBind = unsafe extern "C" fn(
    p_base: sys::GDExtensionTypePtr,
    p_args: *const sys::GDExtensionConstTypePtr,
    r_return: sys::GDExtensionTypePtr,
    p_argument_count: std::os::raw::c_int,
);

pub(crate) type GetUtilityFunction = unsafe extern "C" fn(
    p_function: sys::GDExtensionConstStringNamePtr,
    p_hash: sys::GDExtensionInt,
) -> sys::GDExtensionPtrUtilityFunction;

pub type UtilityFunctionBind = unsafe extern "C" fn(
    r_return: sys::GDExtensionTypePtr,
    p_args: *const sys::GDExtensionConstTypePtr,
    p_argument_count: std::os::raw::c_int,
);

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Utility functions

/// Combination of `as_ref()` and `unwrap_unchecked()`, but without the case differentiation in
/// the former (thus raw pointer access in release mode)
pub(crate) unsafe fn unwrap_ref_unchecked<T>(opt: &Option<T>) -> &T {
    debug_assert_godot!(opt.is_some());

    match opt {
        Some(ref val) => val,
        None => std::hint::unreachable_unchecked(),
    }
}

pub(crate) unsafe fn unwrap_ref_unchecked_mut<T>(opt: &mut Option<T>) -> &mut T {
    debug_assert_godot!(opt.is_some());

    match opt {
        Some(ref mut val) => val,
        None => std::hint::unreachable_unchecked(),
    }
}

pub(crate) fn load_class_method(
    get_method_bind: GetClassMethod,
    string_names: &mut sys::StringCache,
    class_sname_ptr: sys::GDExtensionStringNamePtr,
    class_name: &'static str,
    method_name: &'static str,
    hash: i64,
) -> ClassMethodBind {
    /*crate::out!(
        "Load class method {}::{} (hash {})...",
        class_name,
        method_name,
        hash
    );*/

    // SAFETY: function pointers provided by Godot. We have no way to validate them.
    let method_sname_ptr: sys::GDExtensionStringNamePtr = string_names.fetch(method_name);
    let method: ClassMethodBind =
        unsafe { get_method_bind(class_sname_ptr, method_sname_ptr, hash) };

    if method.is_null() {
        panic!(
            "Failed to load class method {}::{} (hash {}).\n\
            Make sure gdext and Godot are compatible: https://godot-rust.github.io/book/gdext/advanced/compatibility.html",
            class_name,
            method_name,
            hash
        )
    }

    method
}

pub(crate) fn load_builtin_method(
    get_builtin_method: GetBuiltinMethod,
    string_names: &mut sys::StringCache,
    variant_type: sys::GDExtensionVariantType,
    variant_type_str: &'static str,
    method_name: &'static str,
    hash: i64,
) -> BuiltinMethodBind {
    /*crate::out!(
        "Load builtin method {}::{} (hash {})...",
        variant_type,
        method_name,
        hash
    );*/

    // SAFETY: function pointers provided by Godot. We have no way to validate them.
    let method_sname = string_names.fetch(method_name);
    let method = unsafe { get_builtin_method(variant_type, method_sname, hash) };

    method.unwrap_or_else(|| {
        panic!(
            "Failed to load builtin method {variant_type_str}::{method_name} (hash {hash}).{INFO}"
        )
    })
}

pub(crate) fn validate_builtin_lifecycle<T>(function: Option<T>, description: &str) -> T {
    function.unwrap_or_else(|| {
        panic!("Failed to load builtin lifecycle function {description}.{INFO}",)
    })
}

pub(crate) fn load_utility_function(
    get_utility_fn: GetUtilityFunction,
    string_names: &mut sys::StringCache,
    fn_name_str: &'static str,
    hash: i64,
) -> UtilityFunctionBind {
    // SAFETY: function pointers provided by Godot. We have no way to validate them.
    let utility_fn = unsafe { get_utility_fn(string_names.fetch(fn_name_str), hash) };

    utility_fn.unwrap_or_else(|| {
        panic!("Failed to load utility function {fn_name_str} (hash {hash}).{INFO}")
    })
}

const INFO: &str = "\nMake sure gdext and Godot are compatible: https://godot-rust.github.io/book/gdext/advanced/compatibility.html";
