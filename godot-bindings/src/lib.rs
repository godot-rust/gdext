/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

pub(crate) mod watch;

use std::path::Path;

pub use watch::StopWatch;

// Features `custom-godot` and `godot4-artifacts` are mutually exclusive.

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Regenerate all files

#[cfg(feature = "custom-godot")]
#[path = ""]
mod rebuilt {
    use super::*;

    pub(crate) mod godot_exe;
    pub(crate) mod godot_version;
    pub(crate) mod header_gen;

    // #[cfg(not(feature = "custom-godot-extheader"))]
    pub fn load_gdextension_json(watch: &mut StopWatch) -> String {
        godot_exe::load_gdextension_json(watch)
    }

    // #[cfg(not(feature = "custom-godot-extheader"))]
    pub fn write_gdextension_header_rs(to: &Path, watch: &mut StopWatch) {
        godot_exe::load_gdextension_header_rs(None, to, watch);
    }

    #[cfg(feature = "custom-godot-extheader")]
    pub fn write_gdextension_header_rs_from_c(from: &Path, to: &Path, watch: &mut StopWatch) {
        godot_exe::load_gdextension_header_rs(Some(from), to, watch);
    }
}

#[cfg(feature = "custom-godot")]
pub use rebuilt::*;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Reuse existing files

#[cfg(not(feature = "custom-godot"))]
#[path = ""]
mod existing {
    use super::*;

    pub fn load_gdextension_json(_watch: &mut StopWatch) -> &'static str {
        godot4_artifacts::load_gdextension_json()
    }

    pub fn write_gdextension_header_rs(to: &Path, _watch: &mut StopWatch) {
        // Note: prebuilt artifacts just return a static str.
        let header_rs = godot4_artifacts::load_gdextension_header_rs();
        std::fs::write(to, header_rs)
            .unwrap_or_else(|e| panic!("failed to write gdextension_interface.rs: {e}"));
    }
}

#[cfg(not(feature = "custom-godot"))]
pub use existing::*;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Common

pub fn clear_dir(dir: &Path, watch: &mut StopWatch) {
    if dir.exists() {
        std::fs::remove_dir_all(dir).unwrap_or_else(|e| panic!("failed to delete dir: {e}"));
        watch.record("delete_gen_dir");
    }
    std::fs::create_dir_all(dir).unwrap_or_else(|e| panic!("failed to create dir: {e}"));
}
