/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

mod api_parser;
mod central_generator;
mod class_generator;
mod context;
mod interface_generator;
mod special_cases;
mod util;
mod utilities_generator;

#[cfg(test)]
mod tests;

use api_parser::{load_extension_api, ExtensionApi};
use central_generator::{
    generate_core_central_file, generate_core_mod_file, generate_sys_central_file,
    generate_sys_mod_file,
};
use class_generator::{
    generate_builtin_class_files, generate_class_files, generate_native_structures_files,
};
use context::Context;
use interface_generator::generate_sys_interface_file;
use util::{ident, to_pascal_case, to_snake_case};
use utilities_generator::generate_utilities_file;

use proc_macro2::{Ident, TokenStream};
use quote::{quote, ToTokens};
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
    generate_sys_mod_file(sys_gen_path, &mut submit_fn);

    let (api, build_config) = load_extension_api(watch);
    let mut ctx = Context::build_from_api(&api);
    watch.record("build_context");

    generate_sys_central_file(&api, &mut ctx, build_config, sys_gen_path, &mut submit_fn);
    watch.record("generate_central_file");

    let is_godot_4_0 = api.header.version_major == 4 && api.header.version_minor == 0;
    generate_sys_interface_file(h_path, sys_gen_path, is_godot_4_0, &mut submit_fn);
    watch.record("generate_interface_file");
}

pub fn generate_core_files(core_gen_path: &Path) {
    let mut watch = godot_bindings::StopWatch::start();

    generate_core_mod_file(core_gen_path, &mut submit_fn);

    let (api, build_config) = load_extension_api(&mut watch);
    let mut ctx = Context::build_from_api(&api);
    watch.record("build_context");

    generate_core_central_file(&api, &mut ctx, build_config, core_gen_path, &mut submit_fn);
    watch.record("generate_central_file");

    generate_utilities_file(&api, &mut ctx, core_gen_path, &mut submit_fn);
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
// Shared utility types

// Same as above, without lifetimes

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
struct GodotTy {
    ty: String,
    meta: Option<String>,
}

// impl GodotTy {
//     fn new<'a>(ty: &'a String, meta: &'a Option<String>) -> Self {
//         Self {
//             ty: ty.clone(),
//             meta: meta.clone(),
//         }
//     }
// }

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[derive(Clone, Debug)]
enum RustTy {
    /// `bool`, `Vector3i`
    BuiltinIdent(Ident),

    /// `TypedArray<i32>`
    BuiltinArray(TokenStream),

    /// C-style raw pointer to a `RustTy`.
    RawPointer { inner: Box<RustTy>, is_const: bool },

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
    EngineClass {
        tokens: TokenStream,
        #[allow(dead_code)] // currently not read
        class: String,
    },
}

impl RustTy {
    pub fn return_decl(&self) -> TokenStream {
        match self {
            Self::EngineClass { tokens, .. } => quote! { -> Option<#tokens> },
            other => quote! { -> #other },
        }
    }
}

impl ToTokens for RustTy {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            RustTy::BuiltinIdent(ident) => ident.to_tokens(tokens),
            RustTy::BuiltinArray(path) => path.to_tokens(tokens),
            RustTy::RawPointer {
                inner,
                is_const: true,
            } => quote! { *const #inner }.to_tokens(tokens),
            RustTy::RawPointer {
                inner,
                is_const: false,
            } => quote! { *mut #inner }.to_tokens(tokens),
            RustTy::EngineArray { tokens: path, .. } => path.to_tokens(tokens),
            RustTy::EngineEnum { tokens: path, .. } => path.to_tokens(tokens),
            RustTy::EngineClass { tokens: path, .. } => path.to_tokens(tokens),
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Contains multiple naming conventions for types (classes, builtin classes, enums).
#[derive(Clone, Eq, PartialEq, Hash)]
pub(crate) struct TyName {
    godot_ty: String,
    rust_ty: Ident,
}

impl TyName {
    fn from_godot(godot_ty: &str) -> Self {
        Self {
            godot_ty: godot_ty.to_owned(),
            rust_ty: ident(&to_pascal_case(godot_ty)),
        }
    }

    fn description(&self) -> String {
        if self.rust_ty == self.godot_ty {
            self.godot_ty.clone()
        } else {
            format!("{}  [renamed {}]", self.godot_ty, self.rust_ty)
        }
    }

    fn virtual_trait_name(&self) -> String {
        format!("{}Virtual", self.rust_ty)
    }
}

impl ToTokens for TyName {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.rust_ty.to_tokens(tokens)
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Contains naming conventions for modules.
pub(crate) struct ModName {
    // godot_mod: String,
    rust_mod: Ident,
}

impl ModName {
    fn from_godot(godot_ty: &str) -> Self {
        Self {
            // godot_mod: godot_ty.to_owned(),
            rust_mod: ident(&to_snake_case(godot_ty)),
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
    notification_enum_name: Ident,
    has_own_notification_enum: bool,
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
    class_name: TyName,
    module_name: ModName,
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Shared config

// Classes for minimal config
#[cfg(not(feature = "codegen-full"))]
const SELECTED_CLASSES: &[&str] = &[
    "AnimatedSprite2D",
    "ArrayMesh",
    "Area2D",
    "AudioStreamPlayer",
    "BaseButton",
    "Button",
    "BoxMesh",
    "Camera2D",
    "Camera3D",
    "CanvasItem",
    "CanvasLayer",
    "ClassDB",
    "CollisionObject2D",
    "CollisionShape2D",
    "Control",
    "Engine",
    "FileAccess",
    "HTTPRequest",
    "Image",
    "ImageTextureLayered",
    "Input",
    "InputEvent",
    "InputEventAction",
    "Label",
    "MainLoop",
    "Marker2D",
    "Mesh",
    "Node",
    "Node2D",
    "Node3D",
    "Node3DGizmo",
    "Object",
    "OS",
    "PackedScene",
    "PathFollow2D",
    "PhysicsBody2D",
    "PrimitiveMesh",
    "RefCounted",
    "RenderingServer",
    "Resource",
    "ResourceFormatLoader",
    "ResourceLoader",
    "RigidBody2D",
    "SceneTree",
    "Sprite2D",
    "SpriteFrames",
    "TextServer",
    "TextServerExtension",
    "Texture",
    "Texture2DArray",
    "TextureLayered",
    "Time",
    "Timer",
    "Window",
    "Viewport",
];
