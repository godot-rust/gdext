#![macro_use]

pub mod macros;

mod color;
mod others;
mod string;
mod variant;
mod vector2;
mod vector3;

pub use color::*;
pub use others::*;
pub use string::*;
pub use variant::*;
pub use vector2::*;
pub use vector3::*;

pub use glam;

#[macro_export]
macro_rules! gdext_init {
    ($name:ident, $f:expr) => {
        #[no_mangle]
        unsafe extern "C" fn $name(
            interface: *const ::gdext_sys::GDNativeInterface,
            library: ::gdext_sys::GDNativeExtensionClassLibraryPtr,
            init: *mut ::gdext_sys::GDNativeInitialization,
        ) -> ::gdext_sys::GDNativeBool {
            ::gdext_sys::initialize(interface, library);

            let mut handle = $crate::init::InitHandle::new();

            ($f)(&mut handle);

            *init = ::gdext_sys::GDNativeInitialization {
                minimum_initialization_level: handle.lowest_init_level().to_sys(),
                userdata: std::ptr::null_mut(),
                initialize: Some(initialise),
                deinitialize: Some(deinitialise),
            };

            $crate::init::handle = Some(handle);
            true as u8 // TODO allow user to propagate failure
        }


    };
}
