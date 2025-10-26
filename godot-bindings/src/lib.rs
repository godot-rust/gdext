#![cfg_attr(published_docs, feature(doc_cfg))]
/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! # Internal crate of [**godot-rust**](https://godot-rust.github.io)
//!
//! Do not depend on this crate directly, instead use the `godot` crate.
//! No SemVer or other guarantees are provided.

pub(crate) mod watch;

use std::path::Path;

pub use watch::StopWatch;

mod import;

// This is outside of `godot_version` to allow us to use it even when we don't have the `api-custom`
// feature enabled.
#[derive(Eq, PartialEq, Debug)]
pub struct GodotVersion {
    /// the original string (trimmed, stripped of text around)
    pub full_string: String,

    pub major: u8,
    pub minor: u8,

    /// 0 if none
    pub patch: u8,

    /// alpha|beta|dev|stable
    pub status: String,

    /// Git revision 'custom_build.{rev}' or '{official}.rev', if available
    pub custom_rev: Option<String>,
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Custom mode: Regenerate all files

// This file is explicitly included in unit tests. Needs regex dependency.
#[cfg(test)] #[cfg_attr(published_docs, doc(cfg(test)))]
mod godot_version;

#[cfg(feature = "api-custom")] #[cfg_attr(published_docs, doc(cfg(feature = "api-custom")))]
#[path = ""]
mod depend_on_custom {
    use std::borrow::Cow;

    use super::*;

    pub(crate) mod godot_exe;
    pub(crate) mod godot_version;
    pub(crate) mod header_gen;

    pub fn load_gdextension_json(watch: &mut StopWatch) -> Cow<'static, str> {
        Cow::Owned(godot_exe::load_gdextension_json(watch))
    }

    pub fn write_gdextension_headers(h_path: &Path, rs_path: &Path, watch: &mut StopWatch) {
        godot_exe::write_gdextension_headers(h_path, rs_path, false, watch);
    }

    #[cfg(feature = "api-custom-extheader")] #[cfg_attr(published_docs, doc(cfg(feature = "api-custom-extheader")))]
    pub fn write_gdextension_headers_from_c(h_path: &Path, rs_path: &Path, watch: &mut StopWatch) {
        godot_exe::write_gdextension_headers(h_path, rs_path, true, watch);
    }

    pub(crate) fn get_godot_version() -> GodotVersion {
        godot_exe::read_godot_version(&godot_exe::locate_godot_binary())
    }
}

#[cfg(feature = "api-custom")] #[cfg_attr(published_docs, doc(cfg(feature = "api-custom")))]
pub use depend_on_custom::*;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Custom mode: Generate all files based on user provided JSON.

#[cfg(feature = "api-custom-json")] #[cfg_attr(published_docs, doc(cfg(feature = "api-custom-json")))]
#[path = ""]
mod depend_on_custom_json {
    use std::borrow::Cow;

    use super::*;

    pub(crate) mod godot_json;
    pub(crate) mod godot_version;
    pub(crate) mod header_gen;

    pub fn load_gdextension_json(watch: &mut StopWatch) -> Cow<'static, str> {
        let result = godot_json::load_custom_gdextension_json();
        watch.record("read_api_custom_json");
        Cow::Owned(result)
    }

    pub fn write_gdextension_headers(h_path: &Path, rs_path: &Path, watch: &mut StopWatch) {
        godot_json::write_gdextension_headers(h_path, rs_path, watch);
    }

    pub(crate) fn get_godot_version() -> GodotVersion {
        godot_json::read_godot_version()
    }
}

#[cfg(feature = "api-custom-json")] #[cfg_attr(published_docs, doc(cfg(feature = "api-custom-json")))]
pub use depend_on_custom_json::*;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Prebuilt mode: Reuse existing files

#[cfg(not(any(feature = "api-custom", feature = "api-custom-json")))] #[cfg_attr(published_docs, doc(cfg(not(any(feature = "api-custom", feature = "api-custom-json")))))]
#[path = ""]
mod depend_on_prebuilt {
    use super::*;
    use crate::import::prebuilt;

    pub fn load_gdextension_json(_watch: &mut StopWatch) -> std::borrow::Cow<'static, str> {
        prebuilt::load_gdextension_json()
    }

    pub fn write_gdextension_headers(h_path: &Path, rs_path: &Path, watch: &mut StopWatch) {
        // Note: prebuilt artifacts just return a static str.
        let h_contents = prebuilt::load_gdextension_header_h();
        std::fs::write(h_path, h_contents.as_ref())
            .unwrap_or_else(|e| panic!("failed to write gdextension_interface.h: {e}"));
        watch.record("write_header_h");

        let rs_contents = prebuilt::load_gdextension_header_rs();
        std::fs::write(rs_path, rs_contents.as_ref())
            .unwrap_or_else(|e| panic!("failed to write gdextension_interface.rs: {e}"));
        watch.record("write_header_rs");
    }

    pub(crate) fn get_godot_version() -> GodotVersion {
        let version: Vec<&str> = prebuilt::GODOT_VERSION_STRING
            .split('.')
            .collect::<Vec<_>>();

        GodotVersion {
            full_string: prebuilt::GODOT_VERSION_STRING.to_string(),
            major: version[0].parse().unwrap(),
            minor: version[1].parse().unwrap(),
            patch: version
                .get(2)
                .and_then(|patch| patch.parse().ok())
                .unwrap_or(0),
            status: "stable".to_string(),
            custom_rev: None,
        }
    }
}

#[cfg(not(any(feature = "api-custom", feature = "api-custom-json")))] #[cfg_attr(published_docs, doc(cfg(not(any(feature = "api-custom", feature = "api-custom-json")))))]
pub use depend_on_prebuilt::*;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Common

pub fn clear_dir(dir: &Path, watch: &mut StopWatch) {
    if dir.exists() {
        remove_dir_all_reliable(dir);
        watch.record("delete_gen_dir");
    }
    std::fs::create_dir_all(dir).unwrap_or_else(|e| panic!("failed to create dir: {e}"));
}

/// Emit the `cfg` flags for the current Godot version. Allows rustc to know about valid `cfg` values.
pub fn emit_godot_version_cfg() {
    // This could also be done as `KNOWN_API_VERSIONS.len() - 1`, but this is more explicit.
    let all_versions = import::ALL_VERSIONS;

    // Make `published_docs` #[cfg] known. This could be moved to Cargo.toml of all crates in the future.
    println!(r#"cargo:rustc-check-cfg=cfg(published_docs, values(none()))"#);

    // Emit `rustc-check-cfg` for all minor versions (patch .0), so Cargo doesn't complain when we use the #[cfg]s.
    for (_, minor, patch) in all_versions.iter().copied() {
        if minor > 0 && patch == 0 {
            println!(r#"cargo:rustc-check-cfg=cfg(since_api, values("4.{minor}"))"#);
            println!(r#"cargo:rustc-check-cfg=cfg(before_api, values("4.{minor}"))"#);
        }
    }

    let GodotVersion {
        major: _,
        minor,
        patch,
        ..
    } = get_godot_version();

    // Emit `rustc-cfg` dependent on current API version.
    // Start at 1; checking for "since/before 4.0" makes no sense
    let upcoming_minor = all_versions.last().unwrap().1;
    for m in 1..=minor {
        println!(r#"cargo:rustc-cfg=since_api="4.{m}""#);
    }
    for m in minor + 1..=upcoming_minor {
        println!(r#"cargo:rustc-cfg=before_api="4.{m}""#);
    }

    // The below configuration keys are very rarely needed and should generally not be used.
    // Emit #[cfg]s since/before for patch level.
    for (_, m, p) in all_versions.iter().copied() {
        if (m, p) >= (minor, patch) {
            println!(r#"cargo:rustc-cfg=since_patch_api="4.{m}.{p}""#);
        } else {
            println!(r#"cargo:rustc-cfg=before_patch_api="4.{m}.{p}""#);
        }
    }
}

/// Emit `#[cfg(wasm_nothreads)]` flag when compiling to Wasm with the "experimental-wasm-nothreads" feature.
pub fn emit_wasm_nothreads_cfg() {
    println!(r#"cargo:rustc-check-cfg=cfg(wasm_nothreads, values(none()))"#);

    // The environment variable for target family has a list of applicable families separated by commas.
    // For Emscripten in particular, this can be "unix,wasm". Therefore, to check for the Wasm target, we must check each item in the list.
    #[cfg(feature = "experimental-wasm-nothreads")] #[cfg_attr(published_docs, doc(cfg(feature = "experimental-wasm-nothreads")))]
    if std::env::var("CARGO_CFG_TARGET_FAMILY")
        .expect("target family environment variable")
        .split(',')
        .any(|family| family == "wasm")
    {
        println!(r#"cargo:rustc-cfg=wasm_nothreads"#);
    }
}

// Function for safely removal of build directory. Workaround for errors happening during CI builds:
// https://github.com/godot-rust/gdext/issues/616
pub fn remove_dir_all_reliable(path: &Path) {
    let mut retry_count = 0;

    while path.exists() {
        match std::fs::remove_dir_all(path) {
            Ok(_) => break,
            Err(err) => {
                assert_ne!(
                    retry_count,
                    5,
                    "cannot remove directory: {path_display} after 5 tries with error: {err}",
                    path_display = path.display()
                );
                retry_count += 1;
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }
    }
}

/// Concrete check against an API level, not runtime level.
///
/// Necessary in `build.rs`, which doesn't itself have the cfgs.
pub fn before_api(major_minor: &str) -> bool {
    let queried_minor = major_minor
        .strip_prefix("4.")
        .expect("major version must be 4");

    let queried_minor = queried_minor.parse::<u8>().expect("invalid minor version");

    let godot_version = get_godot_version();
    godot_version.minor < queried_minor
}

pub fn since_api(major_minor: &str) -> bool {
    !before_api(major_minor)
}
