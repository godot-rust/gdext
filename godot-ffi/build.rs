/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// See also prebuilt's generator/build.rs which is similar in nature.

use std::path::Path;

fn main() {
    let mut watch = godot_bindings::StopWatch::start();

    // It would be better to generate this in /.generated or /target/godot-gen, however IDEs currently
    // struggle with static analysis when symbols are outside the crate directory (April 2023).
    let gen_path = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gen"));

    // C header is not strictly required, however it is generated for debugging, and to allow CI
    // to check for differences (tweak.patch).
    let h_path = gen_path.join("gdextension_interface.h");
    let rs_path = gen_path.join("gdextension_interface.rs");

    godot_bindings::clear_dir(gen_path, &mut watch);
    godot_bindings::write_gdextension_headers(&h_path, &rs_path, &mut watch);

    godot_codegen::generate_sys_files(gen_path, &h_path, &mut watch);

    watch.write_stats_to(&gen_path.join("ffi-stats.txt"));
    println!("cargo:rerun-if-changed=build.rs");

    godot_bindings::emit_godot_version_cfg();
}
