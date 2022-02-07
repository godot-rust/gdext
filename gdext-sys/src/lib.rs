//! Low level bindings to the provided C core API
#![allow(
    non_camel_case_types,
    non_upper_case_globals,
    non_snake_case,
    deref_nullptr
)]

use std::{cell::RefCell, mem::MaybeUninit};

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

pub unsafe fn set_library(library: GDNativeExtensionClassLibraryPtr) {
    LIBRARY = MaybeUninit::new(library);
}

#[inline(always)]
pub unsafe fn get_library() -> GDNativeExtensionClassLibraryPtr {
    *LIBRARY.as_ptr()
}

#[macro_export]
#[doc(hidden)]
macro_rules! interface_fn {
    ($name:ident) => {{
        fn unoption<T>(x: &Option<T>) -> &T {
            unsafe { std::mem::transmute(x) }
        }
        (unoption(&$crate::get_interface().$name))
    }};
}
