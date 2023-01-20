/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::path::Path;

fn main() {
    let gen_path = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gen/"));

    if gen_path.exists() {
        std::fs::remove_dir_all(gen_path).unwrap_or_else(|e| panic!("failed to delete dir: {e}"));
    }

    // Note: cannot use cfg!(test) because that isn't recognizable from build files.
    // See https://github.com/rust-lang/cargo/issues/1581, which was closed without a solution.
    let stubs_only = cfg!(gdext_test);
    godot_codegen::generate_core_files(gen_path, stubs_only);
}
