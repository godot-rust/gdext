/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::path::Path;

fn main() {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let gen_path = Path::new(&out_dir);

    godot_bindings::remove_dir_all_reliable(gen_path);

    godot_codegen::generate_core_files(gen_path);
    println!("cargo:rerun-if-changed=build.rs");

    godot_bindings::emit_godot_version_cfg();
    godot_bindings::emit_wasm_nothreads_cfg();
    godot_bindings::emit_safeguard_levels();
}
