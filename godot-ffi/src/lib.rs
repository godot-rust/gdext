/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Low level bindings to the provided C core API

#![cfg_attr(test, allow(unused))]

// Output of generated code. Mimics the file structure, symbols are re-exported.
#[rustfmt::skip]
#[allow(
    non_camel_case_types,
    non_upper_case_globals,
    non_snake_case,
    deref_nullptr,
    clippy::redundant_static_lifetimes
)]
pub(crate) mod gen;

mod global_registry;
mod godot_ffi;
mod opaque;
mod plugins;

// See https://github.com/dtolnay/paste/issues/69#issuecomment-962418430
// and https://users.rust-lang.org/t/proc-macros-using-third-party-crate/42465/4
#[doc(hidden)]
pub use paste;

pub use crate::godot_ffi::{GodotFfi, GodotFuncMarshal};
pub use gen::central::*;
pub use gen::gdextension_interface::*;

use crate::global_registry::GlobalRegistry; // needs `crate::`

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Real implementation, when Godot engine is running

struct GodotBinding {
    interface: GDExtensionInterface,
    library: GDExtensionClassLibraryPtr,
    method_table: GlobalMethodTable,
    registry: GlobalRegistry,
}

/// Late-init globals
// Note: static mut is _very_ dangerous. Here a bit less so, since modification happens only once (during init) and no
// &mut references are handed out (except for registry, see below). Overall, UnsafeCell/RefCell + Sync might be a safer abstraction.
static mut BINDING: Option<GodotBinding> = None;

/// # Safety
///
/// - The `interface` pointer must be a valid pointer to a [`GDExtensionInterface`] object.
/// - The `library` pointer must be the pointer given by Godot at initialisation.
/// - This function must not be called from multiple threads.
/// - This function must be called before any use of [`get_library`].
pub unsafe fn initialize(
    interface: *const GDExtensionInterface,
    library: GDExtensionClassLibraryPtr,
) {
    let ver = std::ffi::CStr::from_ptr((*interface).version_string);
    println!(
        "Initialize GDExtension interface: {}",
        ver.to_str().unwrap()
    );

    BINDING = Some(GodotBinding {
        interface: *interface,
        method_table: GlobalMethodTable::new(&*interface),
        registry: GlobalRegistry::default(),
        library,
    });
}

/// # Safety
///
/// The interface must have been initialised with [`initialize`] before calling this function.
#[inline(always)]
pub unsafe fn get_interface() -> &'static GDExtensionInterface {
    &unwrap_ref_unchecked(&BINDING).interface
}

/// # Safety
///
/// The library must have been initialised with [`initialize`] before calling this function.
#[inline(always)]
pub unsafe fn get_library() -> GDExtensionClassLibraryPtr {
    unwrap_ref_unchecked(&BINDING).library
}

/// # Safety
///
/// The interface must have been initialised with [`initialize`] before calling this function.
#[inline(always)]
pub unsafe fn method_table() -> &'static GlobalMethodTable {
    &unwrap_ref_unchecked(&BINDING).method_table
}

/// # Safety
///
/// The interface must have been initialised with [`initialize`] before calling this function.
///
/// Calling this while another place holds a reference (threads, re-entrancy, iteration, etc) is immediate undefined behavior.
// note: could potentially avoid &mut aliasing, using UnsafeCell/RefCell
#[inline(always)]
pub unsafe fn get_registry() -> &'static mut GlobalRegistry {
    &mut unwrap_ref_unchecked_mut(&mut BINDING).registry
}

/// Makes sure that Godot is running, or panics. Debug mode only!
macro_rules! debug_assert_godot {
    ($expr:expr) => {
        debug_assert!(
            $expr,
            "Godot engine not available; make sure you are do not call it from unit/doc tests"
        ); // previous message: "unchecked access to Option::None"
    };
}

/// Combination of `as_ref()` and `unwrap_unchecked()`, but without the case differentiation in
/// the former (thus raw pointer access in release mode)
unsafe fn unwrap_ref_unchecked<T>(opt: &Option<T>) -> &T {
    debug_assert_godot!(opt.is_some());

    match opt {
        Some(ref val) => val,
        None => std::hint::unreachable_unchecked(),
    }
}

unsafe fn unwrap_ref_unchecked_mut<T>(opt: &mut Option<T>) -> &mut T {
    debug_assert_godot!(opt.is_some());

    match opt {
        Some(ref mut val) => val,
        None => std::hint::unreachable_unchecked(),
    }
}

#[doc(hidden)]
#[inline]
pub fn default_call_error() -> GDExtensionCallError {
    GDExtensionCallError {
        error: GDEXTENSION_CALL_OK,
        argument: -1,
        expected: -1,
    }
}

#[doc(hidden)]
#[inline]
pub fn panic_on_call_error(err: &GDExtensionCallError) {
    let actual = err.error;

    assert_eq!(
        actual,
        GDEXTENSION_CALL_OK,
        "encountered Godot error code {}",
        call_error_to_string(actual)
    );
}

fn call_error_to_string(err: GDExtensionCallErrorType) -> &'static str {
    match err {
        GDEXTENSION_CALL_OK => "OK",
        GDEXTENSION_CALL_ERROR_INVALID_METHOD => "ERROR_INVALID_METHOD",
        GDEXTENSION_CALL_ERROR_INVALID_ARGUMENT => "ERROR_INVALID_ARGUMENT",
        GDEXTENSION_CALL_ERROR_TOO_MANY_ARGUMENTS => "ERROR_TOO_MANY_ARGUMENTS",
        GDEXTENSION_CALL_ERROR_TOO_FEW_ARGUMENTS => "ERROR_TOO_FEW_ARGUMENTS",
        GDEXTENSION_CALL_ERROR_INSTANCE_IS_NULL => "ERROR_INSTANCE_IS_NULL",
        GDEXTENSION_CALL_ERROR_METHOD_NOT_CONST => "ERROR_METHOD_NOT_CONST",
        _ => "(unknown)",
    }
}

#[macro_export]
#[doc(hidden)]
macro_rules! builtin_fn {
    ($name:ident $(@1)?) => {
        $crate::method_table().$name
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! builtin_call {
        ($name:ident ( $($args:expr),* $(,)? )) => {
            ($crate::method_table().$name)( $($args),* )
        };
    }

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[macro_export]
#[doc(hidden)]
macro_rules! interface_fn {
    ($name:ident) => {{
        unsafe { $crate::get_interface().$name.unwrap_unchecked() }
    }};
}

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
