/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! # Internal crate of [**godot-rust**](https://godot-rust.github.io)
//!
//! Do not depend on this crate directly, instead use the `godot` crate.
//! No SemVer or other guarantees are provided.

// Codegen has no FFI and thus no reason to use unsafe code.
#![forbid(unsafe_code)]

mod context;
mod conv;
mod formatter;
mod generator;
mod models;
mod special_cases;
mod util;

#[cfg(test)]
mod tests;

use crate::context::Context;
use crate::generator::builtins::generate_builtin_class_files;
use crate::generator::classes::generate_class_files;
use crate::generator::extension_interface::generate_sys_interface_file;
use crate::generator::native_structures::generate_native_structures_files;
use crate::generator::utility_functions::generate_utilities_file;
use crate::generator::{
    generate_core_central_file, generate_core_mod_file, generate_sys_builtin_lifecycle_file,
    generate_sys_builtin_methods_file, generate_sys_central_file, generate_sys_classes_file,
    generate_sys_module_file, generate_sys_utilities_file,
};
use crate::models::domain::{ApiView, ExtensionApi};
use crate::models::json::{load_extension_api, JsonExtensionApi};

use proc_macro2::TokenStream;
use std::path::{Path, PathBuf};

pub type SubmitFn = dyn FnMut(PathBuf, TokenStream);

#[cfg(not(feature = "codegen-full"))]
pub const IS_CODEGEN_FULL: bool = false;

/// Used by itest to determine true codegen status; see itest/build.rs.
#[cfg(feature = "codegen-full")]
pub const IS_CODEGEN_FULL: bool = true;

fn write_file(path: &Path, contents: String) {
    let dir = path.parent().unwrap();
    let _ = std::fs::create_dir_all(dir);

    std::fs::write(path, contents)
        .unwrap_or_else(|e| panic!("failed to write code file to {};\n\t{}", path.display(), e));
}

#[cfg(not(feature = "codegen-rustfmt"))]
fn submit_fn(path: PathBuf, tokens: TokenStream) {
    write_file(&path, formatter::format_tokens(tokens));
}

#[cfg(feature = "codegen-rustfmt")]
mod rustfmt {
    use super::*;
    use std::process::Command;
    use std::sync::Mutex;

    pub fn submit_fn(path: PathBuf, tokens: TokenStream) {
        write_file(&path, tokens.to_string());
        FILES_TO_RUSTFMT.lock().unwrap().push(path);
    }

    pub fn rustfmt_files() {
        let out_files = FILES_TO_RUSTFMT.lock().unwrap();
        println!("Format {} generated files...", out_files.len());

        for files in out_files.chunks(20) {
            let mut command = Command::new("rustfmt");
            command.arg("--edition");
            command.arg("2021");

            for file in files {
                command.arg(file);
            }

            let status = command.status().expect("failed to invoke rustfmt");
            if !status.success() {
                panic!("rustfmt failed on {command:?}");
            }
        }

        println!("Rustfmt completed successfully");
    }

    static FILES_TO_RUSTFMT: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());
}

#[cfg(feature = "codegen-rustfmt")]
pub(crate) use rustfmt::*;

pub fn generate_sys_files(
    sys_gen_path: &Path,
    h_path: &Path,
    watch: &mut godot_bindings::StopWatch,
) {
    let json_api = load_extension_api(watch);

    let mut ctx = Context::build_from_api(&json_api);
    watch.record("build_context");

    let api = ExtensionApi::from_json(&json_api, &mut ctx);
    watch.record("map_domain_models");

    // TODO if ctx is no longer needed for below functions:
    // Deallocate all the JSON models; no longer needed for codegen.
    // drop(json_api);

    generate_sys_central_file(&api, sys_gen_path, &mut submit_fn);
    watch.record("generate_central_file");

    generate_sys_builtin_methods_file(&api, sys_gen_path, &mut ctx, &mut submit_fn);
    watch.record("generate_builtin_methods_file");

    generate_sys_builtin_lifecycle_file(&api, sys_gen_path, &mut submit_fn);
    watch.record("generate_builtin_lifecycle_file");

    generate_sys_classes_file(&api, sys_gen_path, watch, &mut ctx, &mut submit_fn);
    // watch records inside the function.

    generate_sys_utilities_file(&api, sys_gen_path, &mut submit_fn);
    watch.record("generate_utilities_file");

    let is_godot_4_0 = api.godot_version.major == 4 && api.godot_version.minor == 0;
    generate_sys_interface_file(h_path, sys_gen_path, is_godot_4_0, &mut submit_fn);
    watch.record("generate_interface_file");

    generate_sys_module_file(sys_gen_path, &mut submit_fn);
    watch.record("generate_module_file");

    #[cfg(feature = "codegen-rustfmt")]
    {
        rustfmt_files();
        watch.record("rustfmt");
    }
}

pub fn generate_core_files(core_gen_path: &Path) {
    let mut watch = godot_bindings::StopWatch::start();

    generate_core_mod_file(core_gen_path, &mut submit_fn);

    let json_api = load_extension_api(&mut watch);
    let mut ctx = Context::build_from_api(&json_api);
    watch.record("build_context");

    let api = ExtensionApi::from_json(&json_api, &mut ctx);
    let view = ApiView::new(&api);
    watch.record("map_domain_models");

    // TODO if ctx is no longer needed for below functions:
    // Deallocate all the JSON models; no longer needed for codegen.
    // drop(json_api);

    generate_core_central_file(&api, &mut ctx, core_gen_path, &mut submit_fn);
    watch.record("generate_central_file");

    generate_utilities_file(&api, core_gen_path, &mut submit_fn);
    watch.record("generate_utilities_file");

    // Class files -- currently output in godot-core; could maybe be separated cleaner
    // Note: deletes entire generated directory!
    generate_class_files(
        &api,
        &mut ctx,
        &view,
        &core_gen_path.join("classes"),
        &mut submit_fn,
    );
    watch.record("generate_class_files");

    generate_builtin_class_files(
        &api,
        &mut ctx,
        &core_gen_path.join("builtin_classes"),
        &mut submit_fn,
    );
    watch.record("generate_builtin_class_files");

    generate_native_structures_files(
        &api,
        &mut ctx,
        &core_gen_path.join("native"),
        &mut submit_fn,
    );
    watch.record("generate_native_structures_files");

    #[cfg(feature = "codegen-rustfmt")]
    {
        rustfmt_files();
        watch.record("rustfmt");
    }

    watch.write_stats_to(&core_gen_path.join("codegen-stats.txt"));
}
