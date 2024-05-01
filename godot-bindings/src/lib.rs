/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

pub(crate) mod watch;

use std::path::Path;

pub use watch::StopWatch;

#[cfg(feature = "api-4-0")]
use prebuilt_4_0 as godot4_prebuilt;
#[cfg(feature = "api-4-1")]
use prebuilt_4_1 as godot4_prebuilt;

// If none of the api-* features are provided, use default prebuilt version (typically latest Godot stable release).
#[cfg(not(any(
    feature = "api-4-0", //
    feature = "api-4-1", //
    feature = "api-custom", //
)))]
use prebuilt_4_2 as godot4_prebuilt;

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
// Regenerate all files

// This file is explicitly included in unit tests. Needs regex dependency.
#[cfg(test)]
mod godot_version;

#[cfg(feature = "api-custom")]
#[path = ""]
mod custom {
    use super::*;

    pub(crate) mod godot_exe;
    pub(crate) mod godot_version;
    pub(crate) mod header_gen;

    pub fn load_gdextension_json(watch: &mut StopWatch) -> String {
        godot_exe::load_gdextension_json(watch)
    }

    pub fn write_gdextension_headers(h_path: &Path, rs_path: &Path, watch: &mut StopWatch) {
        godot_exe::write_gdextension_headers(h_path, rs_path, false, watch);
    }

    #[cfg(feature = "api-custom-extheader")]
    pub fn write_gdextension_headers_from_c(h_path: &Path, rs_path: &Path, watch: &mut StopWatch) {
        godot_exe::write_gdextension_headers(h_path, rs_path, true, watch);
    }

    pub(crate) fn get_godot_version() -> GodotVersion {
        godot_exe::read_godot_version(&godot_exe::locate_godot_binary())
    }
}

#[cfg(feature = "api-custom")]
pub use custom::*;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Reuse existing files

#[cfg(not(feature = "api-custom"))]
#[path = ""]
mod prebuilt {
    use super::*;

    pub fn load_gdextension_json(_watch: &mut StopWatch) -> &'static str {
        godot4_prebuilt::load_gdextension_json()
    }

    pub fn write_gdextension_headers(h_path: &Path, rs_path: &Path, watch: &mut StopWatch) {
        // Note: prebuilt artifacts just return a static str.
        let h_contents = godot4_prebuilt::load_gdextension_header_h();
        std::fs::write(h_path, h_contents)
            .unwrap_or_else(|e| panic!("failed to write gdextension_interface.h: {e}"));
        watch.record("write_header_h");

        let rs_contents = godot4_prebuilt::load_gdextension_header_rs();
        std::fs::write(rs_path, rs_contents)
            .unwrap_or_else(|e| panic!("failed to write gdextension_interface.rs: {e}"));
        watch.record("write_header_rs");
    }

    pub(crate) fn get_godot_version() -> GodotVersion {
        let version: Vec<&str> = godot4_prebuilt::GODOT_VERSION
            .split('.')
            .collect::<Vec<_>>();
        GodotVersion {
            full_string: godot4_prebuilt::GODOT_VERSION.into(),
            major: version[0].parse().unwrap(),
            minor: version[1].parse().unwrap(),
            patch: version
                .get(2)
                .and_then(|patch| patch.parse().ok())
                .unwrap_or(0),
            status: "stable".into(),
            custom_rev: None,
        }
    }
}

#[cfg(not(feature = "api-custom"))]
pub use prebuilt::*;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Common

const NEXT_MINOR_VERSION: u8 = 3;

pub fn clear_dir(dir: &Path, watch: &mut StopWatch) {
    if dir.exists() {
        remove_dir_all_reliable(dir);
        watch.record("delete_gen_dir");
    }
    std::fs::create_dir_all(dir).unwrap_or_else(|e| panic!("failed to create dir: {e}"));
}

pub fn emit_godot_version_cfg() {
    let GodotVersion {
        major,
        minor,
        patch,
        ..
    } = get_godot_version();

    // Start at 1; checking for "since/before 4.0" makes no sense
    let max = NEXT_MINOR_VERSION;
    for m in 1..=minor {
        println!(r#"cargo:rustc-cfg=since_api="{major}.{m}""#);
    }
    for m in minor + 1..=max {
        println!(r#"cargo:rustc-cfg=before_api="{major}.{m}""#);
    }

    // The below configuration keys are very rarely needed and should generally not be used.
    println!(r#"cargo:rustc-cfg=gdextension_minor_api="{major}.{minor}""#);

    // Godot drops the patch version if it is 0.
    if patch != 0 {
        println!(r#"cargo:rustc-cfg=gdextension_exact_api="{major}.{minor}.{patch}""#);
    } else {
        println!(r#"cargo:rustc-cfg=gdextension_exact_api="{major}.{minor}""#);
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
//
// pub fn write_module_file(path: &Path) {
//     let code = quote! {
//         pub mod table_builtins;
//         pub mod table_builtins_lifecycle;
//         pub mod table_servers_classes;
//         pub mod table_scene_classes;
//         pub mod table_editor_classes;
//         pub mod table_utilities;
//
//         pub mod central;
//         pub mod gdextension_interface;
//         pub mod interface;
//     };
// }
