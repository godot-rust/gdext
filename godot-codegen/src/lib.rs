/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

mod central_generator;
mod class_generator;
mod codegen_special_cases;
mod context;
mod conv;
mod interface_generator;
mod models;
mod special_cases;
mod util;
mod utilities_generator;

#[cfg(test)]
mod tests;

use crate::central_generator::{
    generate_core_central_file, generate_core_mod_file, generate_sys_builtin_lifecycle_file,
    generate_sys_builtin_methods_file, generate_sys_central_file, generate_sys_classes_file,
    generate_sys_utilities_file,
};
use crate::class_generator::{
    generate_builtin_class_files, generate_class_files, generate_native_structures_files,
};
use crate::context::{Context, NotificationEnum};
use crate::interface_generator::generate_sys_interface_file;
use crate::models::domain::{ExtensionApi, TyName};
use crate::models::json::{load_extension_api, JsonExtensionApi};
use crate::util::ident;
use crate::utilities_generator::generate_utilities_file;
use proc_macro2::{Ident, TokenStream};
use quote::ToTokens;
use std::path::{Path, PathBuf};

pub type SubmitFn = dyn FnMut(PathBuf, TokenStream);

fn write_file(path: &Path, contents: String) {
    let dir = path.parent().unwrap();
    let _ = std::fs::create_dir_all(dir);

    std::fs::write(path, contents)
        .unwrap_or_else(|e| panic!("failed to write code file to {};\n\t{}", path.display(), e));
}

#[cfg(feature = "codegen-fmt")]
fn submit_fn(path: PathBuf, tokens: TokenStream) {
    write_file(&path, godot_fmt::format_tokens(tokens));
}

#[cfg(not(feature = "codegen-fmt"))]
fn submit_fn(path: PathBuf, tokens: TokenStream) {
    write_file(&path, tokens.to_string());
}

pub fn generate_sys_files(
    sys_gen_path: &Path,
    h_path: &Path,
    watch: &mut godot_bindings::StopWatch,
) {
    let (json_api, build_config) = load_extension_api(watch);

    let mut ctx = Context::build_from_api(&json_api);
    watch.record("build_context");

    let api = ExtensionApi::from_json(&json_api, build_config, &mut ctx);
    watch.record("map_domain_models");

    // TODO if ctx is no longer needed for below functions:
    // Deallocate all the JSON models; no longer needed for codegen.
    // drop(json_api);

    generate_sys_central_file(&api, &mut ctx, sys_gen_path, &mut submit_fn);
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
}

pub fn generate_core_files(core_gen_path: &Path) {
    let mut watch = godot_bindings::StopWatch::start();

    generate_core_mod_file(core_gen_path, &mut submit_fn);

    let (json_api, build_config) = load_extension_api(&mut watch);
    let mut ctx = Context::build_from_api(&json_api);
    watch.record("build_context");

    let api = ExtensionApi::from_json(&json_api, build_config, &mut ctx);
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
        build_config,
        &core_gen_path.join("classes"),
        &mut submit_fn,
    );
    watch.record("generate_class_files");

    generate_builtin_class_files(
        &api,
        &mut ctx,
        build_config,
        &core_gen_path.join("builtin_classes"),
        &mut submit_fn,
    );
    watch.record("generate_builtin_class_files");

    generate_native_structures_files(
        &api,
        &mut ctx,
        build_config,
        &core_gen_path.join("native"),
        &mut submit_fn,
    );
    watch.record("generate_native_structures_files");

    watch.write_stats_to(&core_gen_path.join("codegen-stats.txt"));
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Contains naming conventions for modules.
#[derive(Clone)]
pub struct ModName {
    // godot_mod: String,
    rust_mod: Ident,
}

impl ModName {
    fn from_godot(godot_ty: &str) -> Self {
        Self {
            // godot_mod: godot_ty.to_owned(),
            rust_mod: ident(&conv::to_snake_case(godot_ty)),
        }
    }
}

impl ToTokens for ModName {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.rust_mod.to_tokens(tokens)
    }
}

struct GeneratedClass {
    code: TokenStream,
    notification_enum: NotificationEnum,
    inherits_macro_ident: Ident,
    /// Sidecars are the associated modules with related enum/flag types, such as `node_3d` for `Node3D` class.
    has_sidecar_module: bool,
}

struct GeneratedBuiltin {
    code: TokenStream,
}

struct GeneratedClassModule {
    class_name: TyName,
    module_name: ModName,
    own_notification_enum_name: Option<Ident>,
    inherits_macro_ident: Ident,
    is_pub_sidecar: bool,
}

struct GeneratedBuiltinModule {
    symbol_ident: Ident,
    module_name: ModName,
}
