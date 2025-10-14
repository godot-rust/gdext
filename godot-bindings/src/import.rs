/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Versions to be updated whenever Godot releases a new patch version we support.
//!
//! This file contains several templating comments, who are substituted by the machinery itest/repo-tweak.
//! When modifying those, make sure to rerun.

/// All stable Godot releases _and_ upcoming next minor release.
pub const ALL_VERSIONS: &[(u8, u8, u8)] = &[
    // [version-sync] [[
    //  [include] past+current+future
    //  [line] \t$triple,
    (4, 2, 0),
    (4, 2, 1),
    (4, 2, 2),
    (4, 3, 0),
    (4, 4, 0),
    (4, 5, 0),
    (4, 6, 0),
    // ]]
];

// [version-sync] [[
//  [line] #[cfg(feature = "api-$kebabVersion")]\npub use gdextension_api::version_$snakeVersion as prebuilt;
#[cfg(feature = "api-4-2")]
pub use gdextension_api::version_4_2 as prebuilt;
#[cfg(feature = "api-4-2-1")]
pub use gdextension_api::version_4_2_1 as prebuilt;
#[cfg(feature = "api-4-2-2")]
pub use gdextension_api::version_4_2_2 as prebuilt;
#[cfg(feature = "api-4-3")]
pub use gdextension_api::version_4_3 as prebuilt;
#[cfg(feature = "api-4-4")]
pub use gdextension_api::version_4_4 as prebuilt;
#[cfg(feature = "api-4-5")]
pub use gdextension_api::version_4_5 as prebuilt;
// ]]

// If none of the api-* features are provided, use default prebuilt version (typically latest Godot stable release).

// [version-sync] [[
//  [line] \tfeature = "api-$kebabVersion",
//  [pre] #[cfg(not(any(
//  [post] \tfeature = "api-custom",\n\tfeature = "api-custom-json",\n)))]
#[cfg(not(any(
    feature = "api-4-2",
    feature = "api-4-2-1",
    feature = "api-4-2-2",
    feature = "api-4-3",
    feature = "api-4-4",
    feature = "api-4-5",
    feature = "api-custom",
    feature = "api-custom-json",
)))]
// ]]
// [version-sync] [[
//  [include] current.minor
//  [line] pub use gdextension_api::version_$snakeVersion as prebuilt;
pub use gdextension_api::version_4_5 as prebuilt;
// ]]

// Cross-compilation support:
// Since godot-bindings is a build-dependency, it and the gdextension-api crate are compiled for the HOST platform.
// The #[cfg] attributes in gdextension-api::load_gdextension_header_rs() evaluate for HOST, not TARGET.
// We read CARGO_CFG_TARGET_* environment variables to select the correct platform-specific Rust bindings at runtime.
#[cfg(not(any(feature = "api-custom", feature = "api-custom-json")))]
pub(crate) mod prebuilt_platform {
    use std::borrow::Cow;
    use std::path::PathBuf;

    /// Load platform-specific Rust bindings (gdextension_interface_{platform}.rs) for the TARGET platform.
    ///
    /// During cross-compilation, godot-bindings runs on the HOST, but needs to generate bindings for the TARGET.
    /// This function reads CARGO_CFG_TARGET_* environment variables to determine the target platform,
    /// then loads the appropriate gdextension_interface_{platform}.rs file from the gdextension-api crate's res/ directory.
    pub fn load_gdextension_header_rs_for_target() -> Cow<'static, str> {
        let target_family = std::env::var("CARGO_CFG_TARGET_FAMILY").ok();
        let target_os = std::env::var("CARGO_CFG_TARGET_OS").ok();

        let platform = match (target_family.as_deref(), target_os.as_deref()) {
            (Some("windows"), _) => "windows",
            (_, Some("macos" | "ios")) => "macos",
            _ => "linux", // Linux, Android, and other Unix-like systems
        };

        load_platform_file(platform)
    }

    /// Reads gdextension_interface_{platform}.rs from the gdextension-api crate using DEP_GDEXTENSION_API_ROOT.
    fn load_platform_file(platform: &str) -> Cow<'static, str> {
        let dep_root = std::env::var("DEP_GDEXTENSION_API_ROOT")
            .expect("DEP_GDEXTENSION_API_ROOT not set. This should be exported by gdextension-api's build script.");

        let file_path = PathBuf::from(dep_root)
            .join("res")
            .join(format!("gdextension_interface_{platform}.rs"));

        std::fs::read_to_string(&file_path)
            .map(Cow::Owned)
            .unwrap_or_else(|e| panic!(
                "Failed to load platform-specific Rust bindings for '{platform}'.\n\
                 Tried to read: {}\n\
                 Error: {e}\n\
                 \n\
                 This is likely a cross-compilation issue or the gdextension-api version doesn't support this platform.",
                file_path.display()
            ))
    }
}
