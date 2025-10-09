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

// Platform-specific header loading for cross-compilation support.
// The prebuilt module is compiled for HOST (not TARGET) when used as a build-dependency,
// so we determine the platform at runtime based on CARGO_CFG_TARGET_* environment variables.
#[cfg(not(any(feature = "api-custom", feature = "api-custom-json")))]
pub(crate) mod prebuilt_platform {
    use std::borrow::Cow;

    /// Load platform-specific prebuilt bindings based on the TARGET platform (not HOST).
    ///
    /// Since godot-bindings is a build-dependency, it's compiled for HOST, but we need bindings for TARGET.
    /// We detect the target platform using CARGO_CFG_TARGET_* environment variables available during
    /// build script execution, then read the corresponding file from the gdextension-api dependency.
    pub fn load_gdextension_header_rs_for_target() -> Cow<'static, str> {
        // Determine TARGET platform from environment variables
        let target_family = std::env::var("CARGO_CFG_TARGET_FAMILY").ok();
        let target_os = std::env::var("CARGO_CFG_TARGET_OS").ok();

        // Select platform suffix matching gdextension-api's file naming
        let platform = match (target_family.as_deref(), target_os.as_deref()) {
            (Some("windows"), _) => "windows",
            (_, Some("macos" | "ios")) => "macos",
            _ => "linux", // Linux, Android, and other Unix-like systems
        };

        // Read the file from gdextension-api dependency in Cargo cache
        load_platform_file_from_cache(platform)
    }

    /// Load platform-specific bindings file from the gdextension-api dependency.
    fn load_platform_file_from_cache(platform: &str) -> Cow<'static, str> {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .expect("HOME or USERPROFILE environment variable");

        // Search git checkouts (for git dependencies)
        if let Some(contents) = try_load_from_git(&home, platform) {
            return Cow::Owned(contents);
        }

        // Search registry (for crates.io dependencies)
        if let Some(contents) = try_load_from_registry(&home, platform) {
            return Cow::Owned(contents);
        }

        panic!(
            "Failed to locate gdextension-api dependency for platform '{}'.\n\
             The dependency should be in Cargo cache at:\n\
             - {}/.cargo/git/checkouts/godot4-prebuilt-*\n\
             - {}/.cargo/registry/src/*/gdextension-api-*",
            platform, home, home
        );
    }

    fn try_load_from_git(home: &str, platform: &str) -> Option<String> {
        let git_dir = format!("{home}/.cargo/git/checkouts");
        for entry in std::fs::read_dir(&git_dir).ok()?.flatten() {
            let path = entry.path();
            if path.file_name()?.to_str()?.starts_with("godot4-prebuilt-") {
                // Found godot4-prebuilt checkout, look for the hash subdirectory
                for subdir in std::fs::read_dir(&path).ok()?.flatten() {
                    let file_path = subdir.path().join(format!(
                        "versions/4.5/res/gdextension_interface_{}.rs",
                        platform
                    ));
                    if let Ok(contents) = std::fs::read_to_string(&file_path) {
                        return Some(contents);
                    }
                }
            }
        }
        None
    }

    fn try_load_from_registry(home: &str, platform: &str) -> Option<String> {
        let registry_dir = format!("{home}/.cargo/registry/src");
        for host_entry in std::fs::read_dir(&registry_dir).ok()?.flatten() {
            for crate_entry in std::fs::read_dir(host_entry.path()).ok()?.flatten() {
                let path = crate_entry.path();
                if path.file_name()?.to_str()?.starts_with("gdextension-api-") {
                    let file_path = path.join(format!(
                        "versions/4.5/res/gdextension_interface_{}.rs",
                        platform
                    ));
                    if let Ok(contents) = std::fs::read_to_string(&file_path) {
                        return Some(contents);
                    }
                }
            }
        }
        None
    }
}
