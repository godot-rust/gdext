/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::path::Path;

fn main() {
    // It would be better to generate this in /.generated or /target/godot-gen, however IDEs currently
    // struggle with static analysis when symbols are outside the crate directory (April 2023).
    let gen_path = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gen"));

    if gen_path.exists() {
        std::fs::remove_dir_all(gen_path).unwrap_or_else(|e| panic!("failed to delete dir: {e}"));
    }

    godot_codegen::generate_core_files(gen_path);
    println!("cargo:rerun-if-changed=build.rs");

    godot_bindings::emit_godot_version_cfg();
}
