/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Low level bindings to the provided C core API

#![cfg_attr(test, allow(unused))]

// Output of generated code. Mimics the file structure, symbols are re-exported.
#[allow(
    non_camel_case_types,
    non_upper_case_globals,
    non_snake_case,
    deref_nullptr,
    clippy::redundant_static_lifetimes
)]
pub(crate) mod gen {
    pub mod central;
    pub mod gdnative_interface;
}

mod global_registry;
mod godot_ffi;
mod opaque;
mod plugins;

// See https://github.com/dtolnay/paste/issues/69#issuecomment-962418430
// and https://users.rust-lang.org/t/proc-macros-using-third-party-crate/42465/4
#[doc(hidden)]
pub use paste;

pub use gen::gdnative_interface::*;
pub use crate::godot_ffi::{GodotFfi, GodotFuncMarshal}; // needs `crate::`

#[cfg(not(test))]
#[doc(inline)]
pub use real_impl::*;

#[cfg(test)]
#[doc(inline)]
pub use test_impl::*;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Real implementation, when Godot engine is running

#[cfg(not(test))]
mod real_impl {
    pub use super::gen::central::*;
    use super::gen::gdnative_interface::*;
    use super::global_registry::GlobalRegistry;

    struct GodotBinding {
        interface: GDNativeInterface,
        library: GDNativeExtensionClassLibraryPtr,
        method_table: GlobalMethodTable,
        registry: GlobalRegistry,
    }

    /// Late-init globals
    // Note: static mut is _very_ dangerous. Here a bit less so, since modification happens only once (during init) and no
    // &mut references are handed out (except for registry, see below). Overall, UnsafeCell/RefCell + Sync might be a safer abstraction.
    static mut BINDING: Option<GodotBinding> = None;

    /// # Safety
    ///
    /// - The `interface` pointer must be a valid pointer to a [`GDNativeInterface`] obj.
    /// - The `library` pointer must be the pointer given by Godot at initialisation.
    /// - This function must not be called from multiple threads.
    /// - This function must be called before any use of [`get_library`].
    pub unsafe fn initialize(
        interface: *const GDNativeInterface,
        library: GDNativeExtensionClassLibraryPtr,
    ) {
        let ver = std::ffi::CStr::from_ptr((*interface).version_string);
        println!(
            "Initialize GDExtension interface: {}",
            ver.to_str().unwrap()
        );
        //dbg!(*interface);

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
    pub unsafe fn get_interface() -> &'static GDNativeInterface {
        &unwrap_ref_unchecked(&BINDING).interface
    }

    /// # Safety
    ///
    /// The library must have been initialised with [`initialize`] before calling this function.
    #[inline(always)]
    pub unsafe fn get_library() -> GDNativeExtensionClassLibraryPtr {
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

    /// Combination of `as_ref()` and `unwrap_unchecked()`, but without the case differentiation in
    /// the former (thus raw pointer access in release mode)
    unsafe fn unwrap_ref_unchecked<T>(opt: &Option<T>) -> &T {
        debug_assert!(opt.is_some(), "unchecked access to Option::None");
        match opt {
            Some(ref val) => val,
            None => std::hint::unreachable_unchecked(),
        }
    }

    unsafe fn unwrap_ref_unchecked_mut<T>(opt: &mut Option<T>) -> &mut T {
        debug_assert!(opt.is_some(), "unchecked access to Option::None");
        match opt {
            Some(ref mut val) => val,
            None => std::hint::unreachable_unchecked(),
        }
    }

    pub fn default_call_error() -> GDNativeCallError {
        GDNativeCallError {
            error: GDNATIVE_CALL_OK,
            argument: -1,
            expected: -1,
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Stubs when in unit-test (without Godot)

#[cfg(test)]
mod test_impl {
    use super::gen::gdnative_interface::*;
    use super::global_registry::GlobalRegistry;

    pub struct GlobalMethodTable {}

    #[inline(always)]
    pub unsafe fn get_interface() -> &'static GDNativeInterface {
        unimplemented!("Not available in unit-tests; needs Godot engine to run.")
    }

    #[inline(always)]
    pub unsafe fn get_library() -> GDNativeExtensionClassLibraryPtr {
        unimplemented!("Not available in unit-tests; needs Godot engine to run.")
    }

    #[inline(always)]
    pub unsafe fn method_table() -> &'static GlobalMethodTable {
        unimplemented!("Not available in unit-tests; needs Godot engine to run.")
    }

    #[inline(always)]
    pub unsafe fn get_registry() -> &'static mut GlobalRegistry {
        unimplemented!("Not available in unit-tests; needs Godot engine to run.")
    }
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
pub fn unbox<T>(value: Box<T>) -> T {
    // Deref-move is a Box magic feature; see https://stackoverflow.com/a/42264074
    *value
}
