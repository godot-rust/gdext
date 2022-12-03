/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

mod api_parser;
mod central_generator;
mod class_generator;
mod context;
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
use context::Context;
use util::ident;
use utilities_generator::generate_utilities_file;
use watch::StopWatch;

use proc_macro2::{Ident, TokenStream};
use quote::{quote, ToTokens};
use std::path::{Path, PathBuf};

pub fn generate_all_files(sys_out_dir: &Path, core_out_dir: &Path, stats_out_dir: &Path) {
    // When invoked by another crate during unit-test (not integration test), don't run generator
    // cfg! is easier to handle than #[cfg] regarding all the imports, and neither code size nor performance matter in unit-test
    if cfg!(feature = "codegen-disabled") {
        return;
    }

    let central_sys_gen_path = sys_out_dir;
    let central_core_gen_path = core_out_dir;
    let class_gen_path = core_out_dir;

    let mut out_files = vec![];

    let mut watch = StopWatch::start();

    let (api, build_config) = load_extension_api(&mut watch);
    let mut ctx = Context::build_from_api(&api);
    watch.record("build_context");

    generate_central_files(
        &api,
        &mut ctx,
        build_config,
        central_sys_gen_path,
        central_core_gen_path,
        &mut out_files,
    );
    watch.record("generate_central_files");

    generate_utilities_file(&api, &mut ctx, class_gen_path, &mut out_files);
    watch.record("generate_utilities_file");

    // Class files -- currently output in godot-core; could maybe be separated cleaner
    // Note: deletes entire generated directory!
    generate_class_files(
        &api,
        &mut ctx,
        build_config,
        &class_gen_path.join("classes"),
        &mut out_files,
    );
    watch.record("generate_class_files");

    rustfmt_if_needed(out_files);
    watch.record("rustfmt");
    watch.write_stats_to(&stats_out_dir.join("codegen-stats.txt"));
}

#[cfg(feature = "codegen-fmt")]
fn rustfmt_if_needed(out_files: Vec<PathBuf>) {
    println!("Format {} generated files...", out_files.len());

    for files in out_files.chunks(20) {
        let mut process = std::process::Command::new("rustup");
        process
            .arg("run")
            .arg("stable")
            .arg("rustfmt")
            .arg("--edition=2021");

        println!("  Format {} files...", files.len());
        for file in files {
            process.arg(file);
        }

        process
            .output()
            .unwrap_or_else(|err| panic!("during godot-rust codegen, rustfmt failed:\n   {err}"));
    }

    println!("Rustfmt completed.");
}

#[cfg(not(feature = "codegen-fmt"))]
fn rustfmt_if_needed(_out_files: Vec<PathBuf>) {}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Shared utility types

#[derive(Clone)]
enum RustTy {
    /// `bool`, `Vector3i`
    BuiltinIdent(Ident),

    /// `TypedArray<i32>`
    BuiltinArray(TokenStream),

    /// `TypedArray<Gd<PhysicsBody3D>>`
    EngineArray {
        tokens: TokenStream,
        #[allow(dead_code)] // only read in minimal config
        elem_class: String,
    },

    /// `module::Enum`
    EngineEnum {
        tokens: TokenStream,
        /// `None` for globals
        #[allow(dead_code)] // only read in minimal config
        surrounding_class: Option<String>,
    },

    /// `Gd<Node>`
    EngineClass(TokenStream),
}

impl RustTy {
    pub fn return_decl(&self) -> TokenStream {
        match self {
            Self::EngineClass(tokens) => quote! { -> Option<#tokens> },
            other => quote! { -> #other },
        }
    }
}

impl ToTokens for RustTy {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            RustTy::BuiltinIdent(ident) => ident.to_tokens(tokens),
            RustTy::BuiltinArray(path) => path.to_tokens(tokens),
            RustTy::EngineArray { tokens: path, .. } => path.to_tokens(tokens),
            RustTy::EngineEnum { tokens: path, .. } => path.to_tokens(tokens),
            RustTy::EngineClass(path) => path.to_tokens(tokens),
            //RustTy::Other(path) => path.to_tokens(tokens),
        }
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

// Classes for minimal config
#[cfg(feature = "minimal")]
const SELECTED_CLASSES: &[&str] = &[
    "AnimatedSprite2D",
    "Area2D",
    "AudioStreamPlayer",
    "BaseButton",
    "Button",
    "Camera2D",
    "Camera3D",
    "CanvasItem",
    "CanvasLayer",
    "CollisionObject2D",
    "CollisionShape2D",
    "Control",
    "Input",
    "OS",
    "FileAccess",
    "Label",
    "MainLoop",
    "Marker2D",
    "Node",
    "Node2D",
    "Node3D",
    "Node3DGizmo",
    "Object",
    "PackedScene",
    "PathFollow2D",
    "PhysicsBody2D",
    "RefCounted",
    "Resource",
    "ResourceLoader",
    "RigidBody2D",
    "SceneTree",
    "Sprite2D",
    "SpriteFrames",
    "Timer",
];
