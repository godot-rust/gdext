/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

#[cfg(feature = "custom-godot")]
pub(crate) mod godot_exe;

#[cfg(feature = "custom-godot")]
pub(crate) mod godot_version;

#[cfg(feature = "custom-godot")]
pub(crate) mod header_gen;

#[cfg(feature = "custom-godot")]
pub(crate) mod watch;

use std::path::Path;
pub use watch::StopWatch;

#[cfg(feature = "custom-godot")]
pub fn load_gdextension_json(watch: &mut StopWatch) -> String {
    godot_exe::load_gdextension_json(watch)
}

#[cfg(feature = "custom-godot")]
pub fn load_gdextension_rust_header(rust_out_path: &Path, watch: &mut StopWatch) -> String {
    godot_exe::load_gdextension_rust_header(rust_out_path, watch)
}
