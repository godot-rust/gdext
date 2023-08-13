/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Functions and macros that are not very specific to gdext, but come in handy.

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
