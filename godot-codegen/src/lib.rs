/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

mod api_parser;
mod central_generator;
mod class_generator;
mod godot_exe;
mod godot_version;
mod special_cases;
mod util;
mod utilities_generator;
mod watch;

#[cfg(test)]
mod tests;

use api_parser::{load_extension_api, ExtensionApi};
use central_generator::generate_central_files;
use class_generator::generate_class_files;
use utilities_generator::generate_utilities_file;

use crate::util::ident;
use crate::watch::StopWatch;
use proc_macro2::{Ident, TokenStream};
use quote::ToTokens;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

pub fn generate_sys_files(sys_out_dir: &Path, core_out_dir: &Path) {
    let central_sys_gen_path = sys_out_dir;
    let central_core_gen_path = core_out_dir;
    let class_gen_path = core_out_dir;

    let mut out_files = vec![];

    let mut watch = StopWatch::start();

    let (api, build_config) = load_extension_api(&mut watch);
    let ctx = build_context(&api);
    watch.record("build_context");

    generate_central_files(
        &api,
        &ctx,
        build_config,
        central_sys_gen_path,
        central_core_gen_path,
        &mut out_files,
    );
    watch.record("generate_central_files");

    generate_utilities_file(&api, &ctx, class_gen_path, &mut out_files);
    watch.record("generate_utilities_file");

    // Class files -- currently output in godot-core; could maybe be separated cleaner
    // Note: deletes entire generated directory!
    generate_class_files(
        &api,
        &ctx,
        build_config,
        &class_gen_path.join("classes"),
        &mut out_files,
    );
    watch.record("generate_class_files");

    rustfmt_if_needed(out_files);
    watch.record("rustfmt");

    watch.write_stats_to(&sys_out_dir.join("build_stats.txt"));
}

fn build_context(api: &ExtensionApi) -> Context {
    let mut ctx = Context::default();

    for class in api.singletons.iter() {
        ctx.singletons.insert(class.name.as_str());
    }

    for class in api.classes.iter() {
        let class_name = class.name.as_str();
        // if !SELECTED_CLASSES.contains(&class_name) {
        //     continue;
        // }

        println!("-- add engine class {}", class_name);
        ctx.engine_classes.insert(class_name);

        if let Some(base) = class.inherits.as_ref() {
            println!("  -- inherits {}", base);
            ctx.inheritance_tree
                .insert(class_name.to_string(), base.clone());
        }
    }
    ctx
}

//#[cfg(feature = "formatted")]
fn rustfmt_if_needed(out_files: Vec<PathBuf>) {
    //print!("Format {} generated files...", out_files.len());

    let mut process = std::process::Command::new("rustup");
    process
        .arg("run")
        .arg("stable")
        .arg("rustfmt")
        .arg("--edition=2021");

    for file in out_files {
        //println!("Format {file:?}");
        process.arg(file);
    }

    match process.output() {
        Ok(_) => println!("Done."),
        Err(err) => {
            println!("Failed.");
            println!("Error: {}", err);
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Shared utility types

struct RustTy {
    tokens: TokenStream,
    is_engine_type: bool,
    is_enum: bool,
}

impl RustTy {
    fn builtin_ident(name: &str) -> Self {
        Self::builtin(ident(name))
    }

    fn builtin(tokens: impl ToTokens) -> Self {
        Self {
            tokens: tokens.to_token_stream(),
            is_engine_type: false,
            is_enum: false,
        }
    }

    fn engine_enum(tokens: impl ToTokens) -> Self {
        Self {
            tokens: tokens.to_token_stream(),
            is_engine_type: true,
            is_enum: true,
        }
    }

    fn engine_class(tokens: impl ToTokens) -> Self {
        Self {
            tokens: tokens.to_token_stream(),
            is_engine_type: true,
            is_enum: false,
        }
    }
}

#[derive(Default)]
pub(crate) struct Context<'a> {
    engine_classes: HashSet<&'a str>,
    singletons: HashSet<&'a str>,
    inheritance_tree: InheritanceTree,
}

impl<'a> Context<'a> {
    fn is_engine_class(&self, class_name: &str) -> bool {
        self.engine_classes.contains(class_name)
    }
    fn is_singleton(&self, class_name: &str) -> bool {
        self.singletons.contains(class_name)
    }
}

#[derive(Default)]
pub(crate) struct InheritanceTree {
    derived_to_base: HashMap<String, String>,
}

impl InheritanceTree {
    pub fn insert(&mut self, derived: String, base: String) {
        let existing = self.derived_to_base.insert(derived, base);
        assert!(existing.is_none(), "Duplicate inheritance insert");
    }

    pub fn map_all_bases<T>(&self, derived: &str, apply: impl Fn(&str) -> T) -> Vec<T> {
        let mut maybe_base = derived;
        let mut result = vec![];
        loop {
            if let Some(base) = self.derived_to_base.get(maybe_base).map(String::as_str) {
                result.push(apply(base));
                maybe_base = base;
            } else {
                break;
            }
        }
        result
    }
}

struct GeneratedClass {
    tokens: TokenStream,
    inherits_macro_ident: Ident,
    has_pub_module: bool,
}

struct GeneratedModule {
    class_ident: Ident,
    module_ident: Ident,
    inherits_macro_ident: Ident,
    is_pub: bool,
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Shared config
// Workaround for limiting number of types as long as implementation is incomplete
/*
const KNOWN_TYPES: &[&str] = &[
    // builtin:
    "bool",
    "int",
    "float",
    "String",
    "StringName",
    "Vector2",
    "Vector2i",
    "Vector3",
    "Vector3i",
    "Vector4",
    "Color",
    "Variant",
    // classes:
    "Object",
    "Node",
    "Node3D",
    "RefCounted",
    "Resource",
    "ResourceLoader",
    "FileAccess",
    "AStar2D",
    "Camera3D",
    "IP",
    "Input",
    "OS",
];

const SELECTED_CLASSES: &[&str] = &[
    "Object",
    "Node",
    "Node3D",
    "RefCounted",
    "Resource",
    "ResourceLoader",
    "FileAccess",
    "AStar2D",
    "Camera3D",
    "IP",
    "Input",
    "OS",
];
*/
