//! Low level bindings to the provided C core API
#![allow(
    non_camel_case_types,
    non_upper_case_globals,
    non_snake_case,
    deref_nullptr,
    clippy::redundant_static_lifetimes
)]

include!(concat!(env!("OUT_DIR"), "/gdnative_interface.rs"));

mod opaque;
mod gen {
    pub(crate) mod central;
}
mod global_registry;
mod godot_ffi;

use gen::central::InterfaceCache;

//pub use opaque::Opaque;
use crate::global_registry::GlobalRegistry;
pub use gen::central::types;
pub use godot_ffi::GodotFfi;

#[allow(non_camel_case_types)]
#[cfg(not(feature = "real_is_double"))]
pub type real = f32;
#[allow(non_camel_case_types)]
#[cfg(feature = "real_is_double")]
pub type real = f64;

// Late-init globals
// TODO maybe they can be combined to a single object? (at least cache + library can for sure)
static mut INTERFACE: Option<GDNativeInterface> = None;
static mut LIBRARY: Option<GDNativeExtensionClassLibraryPtr> = None;
static mut METHOD_TABLE: Option<InterfaceCache> = None;
static mut REGISTRY: Option<GlobalRegistry> = None;

/// # Safety
///
/// The `interface` pointer must be a valid pointer to a [`GDNativeInterface`] object.
pub unsafe fn set_interface(interface: *const GDNativeInterface) {
    INTERFACE = Some(*interface);
    METHOD_TABLE = Some(InterfaceCache::new(&*interface));
    REGISTRY = Some(GlobalRegistry::default());
}

/// # Safety
///
/// The interface must have been initialised with [`set_interface`] before calling this function.
#[inline(always)]
pub unsafe fn get_interface() -> &'static GDNativeInterface {
    unwrap_ref_unchecked(&INTERFACE)
}

/// # Safety
///
/// - The `library` pointer must be the pointer given by Godot at initialisation.
/// - This function must not be called from multiple threads.
/// - This function must be called before any use of [`get_library`].
pub unsafe fn set_library(library: GDNativeExtensionClassLibraryPtr) {
    LIBRARY = Some(library);
}

/// # Safety
///
/// The library must have been initialised with [`set_library`] before calling this function.
#[inline(always)]
pub unsafe fn get_library() -> GDNativeExtensionClassLibraryPtr {
    LIBRARY.unwrap_unchecked()
}

/// # Safety
///
/// The interface must have been initialised with [`set_interface`] before calling this function.
#[inline(always)]
pub unsafe fn get_cache() -> &'static InterfaceCache {
    unwrap_ref_unchecked(&METHOD_TABLE)
}

/// # Safety
///
/// The interface must have been initialised with [`set_interface`] before calling this function.
#[inline(always)]
pub unsafe fn get_registry() -> &'static mut GlobalRegistry {
    unwrap_ref_unchecked_mut(&mut REGISTRY)
}

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
        gdext_sys::static_assert!(std::mem::size_of::<$T>() == std::mem::size_of::<$U>());
    };
    ($T:ty, $U:ty, $msg:literal) => {
        gdext_sys::static_assert!(std::mem::size_of::<$T>() == std::mem::size_of::<$U>(), $msg);
    };
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
