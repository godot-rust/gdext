/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::path::Path;

#[cfg(feature = "custom-godot")]
#[path = ""]
mod rebuilt {
    use super::*;

    pub(crate) mod godot_exe;
    pub(crate) mod godot_version;
    pub(crate) mod header_gen;

    pub fn load_gdextension_json(watch: &mut StopWatch) -> String {
        godot_exe::load_gdextension_json(watch)
    }

    pub fn load_gdextension_header_rs(rust_out_path: &Path, watch: &mut StopWatch) -> String {
        godot_exe::load_gdextension_header_rs(rust_out_path, watch)
    }
}

#[cfg(not(feature = "custom-godot"))]
#[path = ""]
mod existing {
    use super::*;

    pub fn load_gdextension_json(_watch: &mut StopWatch) -> &'static str {
        godot4_artifacts::load_gdextension_json()
    }

    pub fn load_gdextension_header_rs(
        _rust_out_path: &Path,
        _watch: &mut StopWatch,
    ) -> &'static str {
        godot4_artifacts::load_gdextension_header_rs()
    }
}

pub(crate) mod watch;
pub use watch::StopWatch;

#[cfg(feature = "custom-godot")]
pub use rebuilt::*;

#[cfg(not(feature = "custom-godot"))]
pub use existing::*;
