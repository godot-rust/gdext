/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

pub(crate) mod watch;

use std::path::Path;

pub use watch::StopWatch;

// Note: we cannot prevent both `custom-godot` and `prebuilt-godot` from being specified; see Cargo.toml for more information.

#[cfg(not(any(feature = "custom-godot", feature = "prebuilt-godot")))]
compile_error!(
    "At least one of `custom-godot` or `prebuilt-godot` must be specified (none given)."
);

// This is outside of `godot_version` to allow us to use it even when we don't have the `custom-godot`
// feature enabled.
#[derive(Debug, Eq, PartialEq)]
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

#[cfg(feature = "custom-godot")]
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

    #[cfg(feature = "custom-godot-extheader")]
    pub fn write_gdextension_headers_from_c(h_path: &Path, rs_path: &Path, watch: &mut StopWatch) {
        godot_exe::write_gdextension_headers(h_path, rs_path, true, watch);
    }

    pub(crate) fn get_godot_version() -> GodotVersion {
        godot_exe::read_godot_version(&godot_exe::locate_godot_binary())
    }
}

#[cfg(feature = "custom-godot")]
pub use custom::*;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Reuse existing files

#[cfg(not(feature = "custom-godot"))]
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

#[cfg(not(feature = "custom-godot"))]
pub use prebuilt::*;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Common

pub fn clear_dir(dir: &Path, watch: &mut StopWatch) {
    if dir.exists() {
        std::fs::remove_dir_all(dir).unwrap_or_else(|e| panic!("failed to delete dir: {e}"));
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

    println!(r#"cargo:rustc-cfg=gdextension_api="{major}.{minor}""#);

    // Godot drops the patch version if it is 0.
    if patch != 0 {
        println!(r#"cargo:rustc-cfg=gdextension_exact_api="{major}.{minor}.{patch}""#);
    } else {
        println!(r#"cargo:rustc-cfg=gdextension_exact_api="{major}.{minor}""#);
    }
}
