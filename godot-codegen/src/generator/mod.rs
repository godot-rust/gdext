/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::path::Path;

use quote::quote;

use crate::context::Context;
use crate::models::domain::{ClassCodegenLevel, ExtensionApi};
use crate::SubmitFn;

pub mod builtins;
pub mod central_files;
pub mod classes;
pub mod constants;
pub mod default_parameters;
pub mod docs;
pub mod enums;
pub mod extension_interface;
pub mod functions_common;
pub mod gdext_build_struct;
pub mod lifecycle_builtins;
pub mod method_tables;
pub mod native_structures;
pub mod notifications;
pub mod signals;
pub mod utility_functions;
pub mod virtual_definitions;
pub mod virtual_traits;

// ----------------------------------------------------------------------------------------------------------------------------------------------

// Some file generation functions are in specific modules:
// - classes
// - builtins
// - utility_functions
// - native_structures

pub fn generate_sys_module_file(sys_gen_path: &Path, submit_fn: &mut SubmitFn) {
    // Don't delegate #[cfg] to generated code; causes issues in release CI, reproducible with:
    // cargo clippy --features godot/experimental-godot-api,godot/codegen-rustfmt,godot/serde

    let code = quote! {
        pub mod table_builtins;
        pub mod table_builtins_lifecycle;
        pub mod table_core_classes;
        pub mod table_servers_classes;
        pub mod table_scene_classes;
        pub mod table_editor_classes;
        pub mod table_utilities;

        pub mod central;
        pub mod gdextension_interface;
        pub mod interface;
    };

    submit_fn(sys_gen_path.join("mod.rs"), code);
}

pub fn generate_sys_central_file(
    api: &ExtensionApi,
    sys_gen_path: &Path,
    submit_fn: &mut SubmitFn,
) {
    let sys_code = central_files::make_sys_central_code(api);

    submit_fn(sys_gen_path.join("central.rs"), sys_code);
}

pub fn generate_sys_classes_file(
    api: &ExtensionApi,
    sys_gen_path: &Path,
    watch: &mut godot_bindings::StopWatch,
    ctx: &mut Context,
    submit_fn: &mut SubmitFn,
) {
    for api_level in ClassCodegenLevel::with_tables() {
        let code = method_tables::make_class_method_table(api, api_level, ctx);
        let filename = api_level.table_file();

        submit_fn(sys_gen_path.join(filename), code);
        watch.record(format!("generate_classes_{}_file", api_level.lower()));
    }
}

pub fn generate_sys_utilities_file(
    api: &ExtensionApi,
    sys_gen_path: &Path,
    submit_fn: &mut SubmitFn,
) {
    let code = method_tables::make_utility_function_table(api);

    submit_fn(sys_gen_path.join("table_utilities.rs"), code);
}

pub fn generate_sys_builtin_methods_file(
    api: &ExtensionApi,
    sys_gen_path: &Path,
    ctx: &mut Context,
    submit_fn: &mut SubmitFn,
) {
    let code = method_tables::make_builtin_method_table(api, ctx);
    submit_fn(sys_gen_path.join("table_builtins.rs"), code);
}

pub fn generate_sys_builtin_lifecycle_file(
    api: &ExtensionApi,
    sys_gen_path: &Path,
    submit_fn: &mut SubmitFn,
) {
    let code = method_tables::make_builtin_lifecycle_table(api);
    submit_fn(sys_gen_path.join("table_builtins_lifecycle.rs"), code);
}

pub fn generate_core_mod_file(gen_path: &Path, submit_fn: &mut SubmitFn) {
    // When invoked by another crate during unit-test (not integration test), don't run generator.
    let code = quote! {
        pub mod central;
        pub mod classes;
        pub mod builtin_classes;
        pub mod utilities;
        pub mod native;
        pub mod virtuals;
    };

    submit_fn(gen_path.join("mod.rs"), code);
}

pub fn generate_core_central_file(
    api: &ExtensionApi,
    ctx: &mut Context,
    gen_path: &Path,
    submit_fn: &mut SubmitFn,
) {
    let core_code = central_files::make_core_central_code(api, ctx);

    submit_fn(gen_path.join("central.rs"), core_code);
}
