/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Functions and macros that are not very specific to gdext, but come in handy.

use crate as sys;
use std::fmt::{Display, Write};

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

/// Verifies at compile time that two types `T` and `U` have the same size and alignment.
#[macro_export]
macro_rules! static_assert_eq_size_align {
    ($T:ty, $U:ty) => {
        godot_ffi::static_assert!(
            std::mem::size_of::<$T>() == std::mem::size_of::<$U>()
                && std::mem::align_of::<$T>() == std::mem::align_of::<$U>()
        );
    };
    ($T:ty, $U:ty, $msg:literal) => {
        godot_ffi::static_assert!(
            std::mem::size_of::<$T>() == std::mem::size_of::<$U>()
                && std::mem::align_of::<$T>() == std::mem::align_of::<$U>(),
            $msg
        );
    };
}

/// Trace output.
#[cfg(feature = "debug-log")]
#[macro_export]
macro_rules! out {
    ()                          => (eprintln!());
    ($fmt:literal)              => (eprintln!($fmt));
    ($fmt:literal, $($arg:tt)*) => (eprintln!($fmt, $($arg)*));
}

/// Trace output.
#[cfg(not(feature = "debug-log"))]
#[macro_export]
macro_rules! out {
    ()                          => ({});
    ($fmt:literal)              => ({});
    ($fmt:literal, $($arg:tt)*) => {{
        // Discard; should not generate any code.
        if false {
            format_args!($fmt, $($arg)*);
        }
    }}
}

/// Extract a function pointer from its `Option` and convert it to the (dereferenced) target type.
///
/// ```ignore
///  let get_godot_version = get_proc_address(sys::c_str(b"get_godot_version\0"));
///  let get_godot_version = sys::cast_fn_ptr!(get_godot_version as sys::GDExtensionInterfaceGetGodotVersion);
/// ```
///
/// # Safety
///
/// `$ToType` must be an option of an `unsafe extern "C"` function pointer.
#[allow(unused)]
#[macro_export]
macro_rules! unsafe_cast_fn_ptr {
    ($option:ident as $ToType:ty) => {{
        // SAFETY: `$ToType` is an `unsafe extern "C"` function pointer and is thus compatible with `unsafe extern "C" fn()`.
        // And `Option<T>` is compatible with `Option<U>` when both `T` and `U` are compatible function pointers.
        #[allow(unused_unsafe)]
        let ptr: Option<_> = unsafe { std::mem::transmute::<Option<unsafe extern "C" fn()>, $ToType>($option) };
        ptr.expect("null function pointer")
    }};
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

/// Returns a C `const char*` for a null-terminated string slice. UTF-8 encoded.
#[inline]
pub fn c_str_from_str(s: &str) -> *const std::ffi::c_char {
    c_str(s.as_bytes())
}

/// Returns an ad-hoc hash of any object.
pub fn hash_value<T: std::hash::Hash>(t: &T) -> u64 {
    use std::hash::Hasher;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    t.hash(&mut hasher);
    hasher.finish()
}

pub fn join<T, I>(iter: I) -> String
where
    T: std::fmt::Display,
    I: Iterator<Item = T>,
{
    join_with(iter, ", ", |item| format!("{item}"))
}

pub fn join_debug<T, I>(iter: I) -> String
where
    T: std::fmt::Debug,
    I: Iterator<Item = T>,
{
    join_with(iter, ", ", |item| format!("{item:?}"))
}

pub fn join_with<T, I, F, S>(mut iter: I, sep: &str, mut format_elem: F) -> String
where
    I: Iterator<Item = T>,
    F: FnMut(&T) -> S,
    S: Display,
{
    let mut result = String::new();

    if let Some(first) = iter.next() {
        // write! propagates error only if given formatter fails.
        // String formatting by itself is an infallible operation.
        // Read more at: https://doc.rust-lang.org/stable/std/fmt/index.html#formatting-traits
        write!(&mut result, "{first}", first = format_elem(&first))
            .expect("Formatter should not fail!");
        for item in iter {
            write!(&mut result, "{sep}{item}", item = format_elem(&item))
                .expect("Formatter should not fail!");
        }
    }
    result
}

pub fn i64_to_ordering(value: i64) -> std::cmp::Ordering {
    match value {
        -1 => std::cmp::Ordering::Less,
        0 => std::cmp::Ordering::Equal,
        1 => std::cmp::Ordering::Greater,
        _ => panic!("cannot convert value {value} to cmp::Ordering"),
    }
}

/*
pub fn unqualified_type_name<T>() -> &'static str {
    let type_name = std::any::type_name::<T>();
    type_name.split("::").last().unwrap()
}
*/

/// Like [`std::any::type_name`], but returns a short type name without module paths.
pub fn short_type_name<T: ?Sized>() -> String {
    let full_name = std::any::type_name::<T>();
    strip_module_paths(full_name)
}

/// Like [`std::any::type_name_of_val`], but returns a short type name without module paths.
pub fn short_type_name_of_val<T: ?Sized>(val: &T) -> String {
    let full_name = std::any::type_name_of_val(val);
    strip_module_paths(full_name)
}

/// Helper function to strip module paths from a fully qualified type name.
fn strip_module_paths(full_name: &str) -> String {
    let mut result = String::new();
    let mut identifier = String::new();

    let mut chars = full_name.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '<' | '>' | ',' | ' ' | '&' | '(' | ')' | '[' | ']' => {
                // Process the current identifier.
                if !identifier.is_empty() {
                    let short_name = identifier.split("::").last().unwrap_or(&identifier);
                    result.push_str(short_name);
                    identifier.clear();
                }
                result.push(c);

                // Handle spaces after commas for readability.
                if c == ',' && chars.peek().is_some_and(|&next_c| next_c != ' ') {
                    result.push(' ');
                }
            }
            ':' => {
                // Check for '::' indicating module path separator.
                if chars.peek() == Some(&':') {
                    // Skip the second ':'
                    chars.next();
                    identifier.push_str("::");
                } else {
                    identifier.push(c);
                }
            }
            _ => {
                // Part of an identifier.
                identifier.push(c);
            }
        }
    }

    // Process any remaining identifier.
    if !identifier.is_empty() {
        let short_name = identifier.split("::").last().unwrap_or(&identifier);
        result.push_str(short_name);
    }

    result
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Private helpers

/// Metafunction to extract inner function pointer types from all the bindgen `Option<F>` type names.
/// Needed for `unsafe_cast_fn_ptr` macro.
pub trait Inner: Sized {
    type FnPtr: Sized;
}

impl<T> Inner for Option<T> {
    type FnPtr = T;
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Function types used for table loaders

pub(crate) type GetClassMethod = unsafe extern "C" fn(
    p_classname: sys::GDExtensionConstStringNamePtr,
    p_methodname: sys::GDExtensionConstStringNamePtr,
    p_hash: sys::GDExtensionInt,
) -> sys::GDExtensionMethodBindPtr;

/// Newtype around `GDExtensionMethodBindPtr` so we can implement `Sync` and `Send` for it manually.    
#[derive(Copy, Clone)]
pub struct ClassMethodBind(pub sys::GDExtensionMethodBindPtr);

// SAFETY: `sys::GDExtensionMethodBindPtr` is effectively the same as a `unsafe extern "C" fn`. So sharing it between
// threads is fine, as using it in any way requires `unsafe` and it is up to the caller to ensure it is thread safe
// to do so.
unsafe impl Sync for ClassMethodBind {}
// SAFETY: See `Sync` impl safety doc.
unsafe impl Send for ClassMethodBind {}

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

// TODO: Most of these should be `unsafe` since the caller passes an `unsafe extern "C"` function pointer which it must be legal to call.
// But for now we can just rely on knowing that these aren't called in the wrong context.

pub(crate) fn load_class_method(
    get_method_bind: GetClassMethod,
    string_names: &mut sys::StringCache,
    class_sname_ptr: Option<sys::GDExtensionStringNamePtr>,
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

    let method_sname_ptr: sys::GDExtensionStringNamePtr = string_names.fetch(method_name);
    let class_sname_ptr = class_sname_ptr.unwrap_or_else(|| string_names.fetch(class_name));

    // SAFETY: function pointers provided by Godot. We have no way to validate them.
    let method: sys::GDExtensionMethodBindPtr =
        unsafe { get_method_bind(class_sname_ptr, method_sname_ptr, hash) };

    if method.is_null() {
        panic!("Failed to load class method {class_name}::{method_name} (hash {hash}).{INFO}")
    }

    ClassMethodBind(method)
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

    let method_sname = string_names.fetch(method_name);
    // SAFETY: function pointers provided by Godot. We have no way to validate them.
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

pub(crate) fn read_version_string(version_ptr: &sys::GDExtensionGodotVersion) -> String {
    let char_ptr = version_ptr.string;

    // SAFETY: GDExtensionGodotVersion has the (manually upheld) invariant of a valid string field.
    let c_str = unsafe { std::ffi::CStr::from_ptr(char_ptr) };

    let full_version = c_str.to_str().unwrap_or("(invalid UTF-8 in version)");

    full_version
        .strip_prefix("Godot Engine ")
        .unwrap_or(full_version)
        .to_string()
}

const INFO: &str = "\nMake sure gdext and Godot are compatible: https://godot-rust.github.io/book/toolchain/compatibility.html";

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Private abstractions
// Don't use abstractions made here outside this crate, if needed then we should discuss making it more of a first-class
// abstraction like `godot-cell`.

/// Module to encapsulate `ManualInitCell`.
mod manual_init_cell {
    use std::cell::UnsafeCell;
    use std::hint::unreachable_unchecked;

    /// A cell which can be initialized and uninitialized, with manual synchronization from the caller.
    ///
    /// Similar to a [`OnceLock`](std::sync::OnceLock), but without the overhead of locking for initialization. In most cases the compiler
    /// seems able to optimize `OnceLock` to equivalent code. But this guaranteed does not have any overhead at runtime.
    ///
    /// This cell additionally allows to deinitialize the value. Access to uninitialized values is UB, but checked in Debug mode.
    pub(crate) struct ManualInitCell<T> {
        // Invariant: Is `None` until initialized, and then never modified after (except, possibly, through interior mutability).
        cell: UnsafeCell<Option<T>>,
    }

    impl<T> ManualInitCell<T> {
        /// Creates a new empty cell.
        pub const fn new() -> Self {
            Self {
                cell: UnsafeCell::new(None),
            }
        }

        /// Initialize the value stored in this cell.
        ///
        /// # Safety
        ///
        /// - Must only be called once, unless a [`clear()`][Self::clear] call has happened in between.
        /// - Calls to this method must not happen concurrently with a call to any other method on this cell.
        ///
        /// Note that the other methods of this cell do not have a safety invariant that they are not called concurrently with `set`.
        /// This is because doing so would violate the safety invariants of `set` and so they do not need to explicitly have that as a
        /// safety invariant as well. This has the added benefit that `is_initialized` can be a safe method.
        #[inline]
        pub unsafe fn set(&self, value: T) {
            // SAFETY: `set` has exclusive access to the cell, per the safety requirements.
            let option = unsafe { &mut *self.cell.get() };

            // Tell the compiler that the cell is `None`, even if it can't prove that on its own.
            if option.is_some() {
                // SAFETY: `set` cannot be called multiple times without `clear` in between, so the cell must be `None` at this point.
                // This panics in Debug mode.
                unsafe { unreachable_unchecked() }
            }

            *option = Some(value);
        }

        /// Clear the value stored in this cell.
        ///
        /// # Safety
        ///
        /// - Must only be called after [`set`](Self::set) has been called.
        /// - Calls to this method must not happen concurrently with a call to any other method on this cell.
        #[inline]
        pub unsafe fn clear(&self) {
            // SAFETY: `set` is only ever called once, and is not called concurrently with any other methods. Therefore, we can take
            // a mutable reference to the contents of the cell.
            let option = unsafe { &mut *self.cell.get() };

            // Tell the compiler that the cell is `Some`.
            if option.is_none() {
                // SAFETY: `set` has been called before this, so the option is known to be a `Some`.
                // This panics in Debug mode.
                unsafe { unreachable_unchecked() }
            }

            *option = None;
        }

        /// Gets the value stored in the cell.
        ///
        /// # Safety
        ///
        /// - [`set`](ManualInitCell::set) must have been called before calling this method.
        #[inline]
        pub unsafe fn get_unchecked(&self) -> &T {
            // SAFETY: There are no `&mut` references, since only `set` can create one and this method cannot be called concurrently with `set`.
            let option = unsafe { &*self.cell.get() };

            // SAFETY: `set` has been called before this, so the option is known to be a `Some`.
            // This panics in Debug mode.
            unsafe { option.as_ref().unwrap_unchecked() }
        }

        /// Checks whether the cell contains a value.
        #[inline]
        pub fn is_initialized(&self) -> bool {
            // SAFETY: There are no `&mut` references, since only `set` can create one and this method cannot be called concurrently with `set`.
            let option = unsafe { &*self.cell.get() };

            option.is_some()
        }
    }

    // SAFETY: The user is responsible for ensuring thread safe initialization of the cell.
    // This also requires `Send` for the same reasons `OnceLock` does.
    unsafe impl<T: Send + Sync> Sync for ManualInitCell<T> {}
    // SAFETY: See `Sync` impl.
    unsafe impl<T: Send> Send for ManualInitCell<T> {}
}

pub(crate) use manual_init_cell::ManualInitCell;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Unit tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_short_type_name() {
        assert_eq!(short_type_name::<i32>(), "i32");
        assert_eq!(short_type_name::<Option<i32>>(), "Option<i32>");
        assert_eq!(
            short_type_name::<Result<Option<i32>, String>>(),
            "Result<Option<i32>, String>"
        );
        assert_eq!(
            short_type_name::<Vec<Result<Option<i32>, String>>>(),
            "Vec<Result<Option<i32>, String>>"
        );
        assert_eq!(
            short_type_name::<std::collections::HashMap<String, Vec<i32>>>(),
            "HashMap<String, Vec<i32>>"
        );
        assert_eq!(
            short_type_name::<Result<Option<i32>, String>>(),
            "Result<Option<i32>, String>"
        );
        assert_eq!(short_type_name::<i32>(), "i32");
        assert_eq!(short_type_name::<Vec<String>>(), "Vec<String>");
    }

    #[test]
    fn test_short_type_name_of_val() {
        let value = Some(42);
        assert_eq!(short_type_name_of_val(&value), "Option<i32>");

        let result: Result<_, String> = Ok(Some(42));
        assert_eq!(
            short_type_name_of_val(&result),
            "Result<Option<i32>, String>"
        );

        let vec = vec![result];
        assert_eq!(
            short_type_name_of_val(&vec),
            "Vec<Result<Option<i32>, String>>"
        );
    }
}
