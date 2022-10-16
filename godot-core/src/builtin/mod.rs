/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

#![macro_use]

pub mod macros;

mod arrays;
mod color;
mod others;
mod string;
mod variant;
mod vector2;
mod vector3;
mod vector4;

pub use arrays::*;
pub use color::*;
pub use others::*;
pub use string::*;
pub use variant::*;
pub use vector2::*;
pub use vector3::*;
pub use vector4::*;

pub use glam;

#[macro_export]
macro_rules! gdext_init {
    ($name:ident, $f:expr) => {
        #[no_mangle]
        unsafe extern "C" fn $name(
            interface: *const ::godot_ffi::GDNativeInterface,
            library: ::godot_ffi::GDNativeExtensionClassLibraryPtr,
            init: *mut ::godot_ffi::GDNativeInitialization,
        ) -> ::godot_ffi::GDNativeBool {
            ::godot_ffi::initialize(interface, library);

            let mut handle = $crate::init::InitHandle::new();

            ($f)(&mut handle);

            *init = ::godot_ffi::GDNativeInitialization {
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
