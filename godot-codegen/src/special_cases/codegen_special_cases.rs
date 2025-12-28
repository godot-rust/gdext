/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Codegen-dependent exclusions. Can be removed if feature `codegen-full` is removed.

// TODO make this file private and only accessed by special_cases.rs.

use crate::context::Context;
use crate::models::json::{JsonBuiltinMethod, JsonClassMethod, JsonUtilityFunction};
use crate::special_cases;

pub(crate) fn is_builtin_method_excluded(_method: &JsonBuiltinMethod) -> bool {
    // Prior to Godot 4.2, builtin varargs (method.is_vararg) weren't supported, but that's now our minimum supported version.

    false
}

#[cfg(not(feature = "codegen-full"))]
pub(crate) fn is_class_excluded(godot_class_name: &str) -> bool {
    !SELECTED_CLASSES.contains(&godot_class_name)
}

#[cfg(feature = "codegen-full")]
pub(crate) fn is_class_excluded(_godot_class_name: &str) -> bool {
    false
}

#[cfg(not(feature = "codegen-full"))]
pub(crate) fn is_native_struct_excluded(native_struct: &str) -> bool {
    native_struct == "CaretInfo"
}

#[cfg(feature = "codegen-full")]
pub(crate) fn is_native_struct_excluded(_native_struct: &str) -> bool {
    false
}

#[cfg(not(feature = "codegen-full"))]
fn is_type_excluded(ty: &str, ctx: &mut Context) -> bool {
    use crate::conv;
    use crate::models::domain::RustTy;

    fn is_rust_type_excluded(ty: &RustTy) -> bool {
        match ty {
            RustTy::BuiltinIdent { .. } => false,
            RustTy::BuiltinArray { .. } => false,
            RustTy::GenericArray => false,
            RustTy::RawPointer { inner, .. } => is_rust_type_excluded(inner),
            RustTy::SysPointerType { .. } => true,
            RustTy::EngineArray { elem_class, .. } => is_class_excluded(elem_class.as_str()),
            RustTy::EngineEnum {
                surrounding_class, ..
            } => match surrounding_class.as_ref() {
                None => false,
                Some(class) => is_class_excluded(class.as_str()),
            },
            RustTy::EngineClass { inner_class, .. } => is_class_excluded(&inner_class.to_string()),
            RustTy::ExtenderReceiver { .. } => false,
        }
    }

    // Both meta + flow direction are irrelevant here.
    is_rust_type_excluded(&conv::to_rust_temporary_type(ty, ctx))
}

#[cfg(feature = "codegen-full")]
fn is_type_excluded(_ty: &str, _ctx: &mut Context) -> bool {
    false
}

pub(crate) fn is_class_method_excluded(method: &JsonClassMethod, ctx: &mut Context) -> bool {
    let is_arg_or_return_excluded = |ty: &str, _ctx: &mut Context| {
        // First check if the type is explicitly deleted. In Godot, type names are unique without further categorization,
        // so passing in a class name while checking for any types is fine.
        let class_deleted = special_cases::is_godot_type_deleted(ty);

        // Then also check if the type is excluded from codegen (due to current Cargo feature). RHS is always false in full-codegen.
        class_deleted || is_type_excluded(ty, _ctx)
    };

    // Exclude if return type contains an excluded type.
    if method
        .return_value
        .as_ref()
        .is_some_and(|ret| is_arg_or_return_excluded(ret.type_.as_str(), ctx))
    {
        return true;
    }

    // Exclude if any argument contains an excluded type.
    if method.arguments.as_ref().is_some_and(|args| {
        args.iter()
            .any(|arg| is_arg_or_return_excluded(arg.type_.as_str(), ctx))
    }) {
        return true;
    }

    false
}

#[cfg(feature = "codegen-full")]
pub(crate) fn is_utility_function_excluded(
    _function: &JsonUtilityFunction,
    _ctx: &mut Context,
) -> bool {
    false
}

#[cfg(not(feature = "codegen-full"))]
pub(crate) fn is_utility_function_excluded(
    function: &JsonUtilityFunction,
    ctx: &mut Context,
) -> bool {
    function
        .return_type
        .as_ref()
        .is_some_and(|ret| is_type_excluded(ret.as_str(), ctx))
        || function.arguments.as_ref().is_some_and(|args| {
            args.iter()
                .any(|arg| is_type_excluded(arg.type_.as_str(), ctx))
        })
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Allowed-classes

// Classes for minimal config
#[cfg(not(feature = "codegen-full"))]
const SELECTED_CLASSES: &[&str] = &[
    // Core class hierarchy
    "Object",
    "Node",
    "CanvasItem", // base of Node2D
    "Node2D",
    "Node3D",
    "RefCounted",
    "Resource",
    //
    // Runtime + reflection support
    "ClassDB",
    "Engine",
    "OS",
    //
    // Editor plugins
    "EditorPlugin",
    "EditorExportPlugin",
    //
    // I/O and save/load
    "ResourceLoader",
    "ResourceSaver",
    "FileAccess",
    //
    // Scene (node_test, rpc_test)
    "MainLoop", // base of SceneTree
    "SceneTree",
    //
    // Script instances
    "Script",
    "ScriptExtension",
    "ScriptNameCasing",
    "ScriptLanguage",
    "ScriptLanguageExtension",
    "GDScript",
    //
    // Example resources
    "PackedScene", // manual_extensions
    //
    // Meshes (virtual_methods_test)
    "Mesh",
    "ArrayMesh", // enum_test, 1 case, but small API
    "PrimitiveMesh",
    //
    // Windowing + Input (virtual_methods_test)
    "Viewport",
    "Window",
    "Input",
    "InputEvent",
    "InputEventAction",
    //
    // Godot servers (for RID support)
    "RenderingServer",
    //
    // Misc
    "Time", // usage: enum_test.enum_hash()
    "HTTPRequest",
    "ResourceFormatLoader", // TODO: replace?
];
