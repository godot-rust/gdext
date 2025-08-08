/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use proc_macro2::TokenStream;
use quote::quote;

use crate::models::domain::GodotApiVersion;

pub fn make_gdext_build_struct(header: &GodotApiVersion) -> TokenStream {
    let GodotApiVersion {
        major,
        minor,
        patch,
        version_string,
    } = header;

    // Should this be mod?
    quote! {
        /// Provides meta-information about the library and the Godot version in use.
        pub struct GdextBuild;

        impl GdextBuild {
            /// Godot version against which gdext was compiled.
            ///
            /// Example format: `v4.0.stable.official`
            pub const fn godot_static_version_string() -> &'static str {
                #version_string
            }

            /// Godot version against which gdext was compiled, as `(major, minor, patch)` triple.
            pub const fn godot_static_version_triple() -> (u8, u8, u8) {
                (#major, #minor, #patch)
            }

            /// Version of the Godot engine which loaded gdext via GDExtension binding.
            pub fn godot_runtime_version_string() -> String {
                unsafe {
                    let char_ptr = crate::runtime_metadata().godot_version.string;
                    let c_str = std::ffi::CStr::from_ptr(char_ptr);
                    String::from_utf8_lossy(c_str.to_bytes()).to_string()
                }
            }

            /// Version of the Godot engine which loaded gdext via GDExtension binding, as
            /// `(major, minor, patch)` triple.
            pub fn godot_runtime_version_triple() -> (u8, u8, u8) {
                let version = unsafe {
                    crate::runtime_metadata().godot_version
                };
                (version.major as u8, version.minor as u8, version.patch as u8)
            }

            // Duplicates code from `before_api` in `godot-bindings/lib.rs`.

            /// For a string `"4.x"`, returns `true` if the current Godot version is strictly less than 4.x.
            ///
            /// Runtime equivalent of `#[cfg(before_api = "4.x")]`.
            ///
            /// # Panics
            /// On bad input.
            pub fn before_api(major_minor: &str) -> bool {
                let mut parts = major_minor.split('.');
                let queried_major = parts.next().unwrap().parse::<u8>().expect("invalid major version");
                let queried_minor = parts.next().unwrap().parse::<u8>().expect("invalid minor version");
                assert_eq!(queried_major, 4, "major version must be 4");

                let (_, minor, _) = Self::godot_runtime_version_triple();
                minor < queried_minor
            }

            /// For a string `"4.x"`, returns `true` if the current Godot version is equal or greater to 4.x.
            ///
            /// Runtime equivalent of `#[cfg(since_api = "4.x")]`.
            ///
            /// # Panics
            /// On bad input.
            pub fn since_api(major_minor: &str) -> bool {
                !Self::before_api(major_minor)
            }
        }
    }
}
