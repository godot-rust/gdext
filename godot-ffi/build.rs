/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::path::Path;

fn main() {
    let mut watch = godot_input::StopWatch::start();

    let gen_path = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gen/"));
    if gen_path.exists() {
        std::fs::remove_dir_all(gen_path).unwrap_or_else(|e| panic!("failed to delete dir: {e}"));
        watch.record("delete_gen_dir");
    }
    std::fs::create_dir_all(gen_path).unwrap_or_else(|e| panic!("failed to create dir: {e}"));

    let rust_header_path = gen_path.join("gdextension_interface.rs");
    let header = godot_input::load_gdextension_header_rs(&rust_header_path, &mut watch);
    std::fs::write(rust_header_path, header).expect("failed to write extension header");

    godot_codegen::generate_sys_files(gen_path, &mut watch);
    watch.write_stats_to(&gen_path.join("godot-ffi-stats.txt"));
}
