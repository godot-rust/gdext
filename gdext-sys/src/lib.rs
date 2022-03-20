//! Low level bindings to the provided C core API
#![allow(
    non_camel_case_types,
    non_upper_case_globals,
    non_snake_case,
    deref_nullptr,
    clippy::redundant_static_lifetimes
)]

use std::mem::MaybeUninit;

include!(concat!(env!("OUT_DIR"), "/gdnative_interface.rs"));

#[allow(non_camel_case_types)]
#[cfg(feature = "real_is_float")]
pub type real = f32;
#[allow(non_camel_case_types)]
#[cfg(feature = "real_is_double")]
pub type real = f64;

static mut INTERFACE: MaybeUninit<GDNativeInterface> = MaybeUninit::uninit();

static mut LIBRARY: MaybeUninit<GDNativeExtensionClassLibraryPtr> = MaybeUninit::uninit();

/// # Safety
///
/// The `interface` pointer must be a valid pointer to a [`GDNativeInterface`] object.
pub unsafe fn set_interface(interface: *const GDNativeInterface) {
    INTERFACE = MaybeUninit::new(*interface);
}

/// # Safety
///
/// The interface must have been initialised with [`set_interface`] before calling this function.
#[inline(always)]
pub unsafe fn get_interface() -> &'static GDNativeInterface {
    &*INTERFACE.as_ptr()
}

/// # Safety
///
/// - The `library` pointer must be the pointer given by Godot at initialisation.
/// - This function must not be called from multiple threads.
/// - This funnction must be called before any use of [`get_library`].
pub unsafe fn set_library(library: GDNativeExtensionClassLibraryPtr) {
    LIBRARY = MaybeUninit::new(library);
}

/// # Safety
///
/// The library must have been initialised with [`set_library`] before calling this function.
#[inline(always)]
pub unsafe fn get_library() -> GDNativeExtensionClassLibraryPtr {
    *LIBRARY.as_ptr()
}

#[macro_export]
#[doc(hidden)]
macro_rules! interface_fn {
    ($name:ident) => {{
        unsafe { $crate::get_interface().$name.unwrap_unchecked() }
    }};
}
