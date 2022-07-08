mod api_parser;
mod central_generator;
mod class_generator;
mod godot_exe;
mod godot_version;
mod util;

use crate::api_parser::ExtensionApi;
use api_parser::load_extension_api;
use central_generator::generate_central_file;
use class_generator::generate_class_files;
use proc_macro2::TokenStream;
use std::collections::{HashMap, HashSet};
use std::env;
use std::path::{Path, PathBuf};

// macro_rules! local_path {
//     ($path:lit) => {
//         Path::new(concat!(env!("CARGO_MANIFEST_DIR"), $path))
//     };
// }

pub fn generate() {
    let sys_gen_path = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../gdext-sys/src/gen"));
    let class_gen_path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../gdext-class/src/gen"
    ));

    let mut out_files = vec![];

    let now = std::time::Instant::now();
    let (api, build_config) = load_extension_api();
    let load_time = now.elapsed().as_millis();

    let ctx = build_context(&api);

    let now = std::time::Instant::now();
    generate_central_file(&api, &ctx, build_config, sys_gen_path, &mut out_files);
    let central_time = now.elapsed().as_millis();

    // Class files -- currently output in gdext-class; could maybe be separated cleaner
    let now = std::time::Instant::now();
    // Note: deletes entire generated directory!
    generate_class_files(
        &api,
        &ctx,
        build_config,
        &class_gen_path.join("classes"),
        &mut out_files,
    );
    let class_time = now.elapsed().as_millis();

    let now = std::time::Instant::now();
    rustfmt_if_needed(out_files);
    let fmt_time = now.elapsed().as_millis();

    println!("Times [ms]:");
    println!("  load-json:     {load_time}");
    println!("  gen-central:   {central_time}");
    println!("  gen-class:     {class_time}");
    println!("  fmt:           {fmt_time}");
}

fn build_context(api: &ExtensionApi) -> Context {
    let mut ctx = Context::default();
    for class in api.classes.iter() {
        let class_name = class.name.as_str();
        if !SELECTED_CLASSES.contains(&class_name) {
            continue;
        }

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
    is_engine_class: bool,
}

#[derive(Default)]
struct Context<'a> {
    engine_classes: HashSet<&'a str>,
    inheritance_tree: InheritanceTree,
}

impl<'a> Context<'a> {
    fn is_engine_class(&self, class_name: &str) -> bool {
        self.engine_classes.contains(class_name)
    }
}

#[derive(Default)]
struct InheritanceTree {
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

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Shared config
// Workaround for limiting number of types as long as implementation is incomplete
const KNOWN_TYPES: [&str; 11] = [
    // builtin:
    "bool",
    "int",
    "float",
    "String",
    "Vector2",
    "Vector3",
    "Color",
    // classes:
    "Object",
    "Node",
    "Node3D",
    "RefCounted",
];

const SELECTED_CLASSES: [&str; 4] = ["Object", "Node", "Node3D", "RefCounted"];
