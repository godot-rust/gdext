/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// Lists all cases in the Godot class API, where deviations are considered appropriate (e.g. for safety).

// Naming:
// * Class methods:             is_class_method_*
// * Builtin methods:           is_builtin_method_*
// * Class or builtin methods:  is_method_*

// Open design decisions:
// * Should Godot types like Node3D have all the "obj level" methods like to_string(), get_instance_id(), etc; or should those
//   be reserved for the Gd<T> pointer? The latter seems like a limitation. User objects also have to_string() (but not get_instance_id())
//   through the GodotExt trait. This could be unified.
// * The deleted/private methods and classes deemed "dangerous" may be provided later as unsafe functions -- our safety model
//   needs to first mature a bit.

// NOTE: the methods are generally implemented on Godot types (e.g. AABB, not Aabb)

#![allow(clippy::match_like_matches_macro)] // if there is only one rule

use crate::models::domain::TyName;
use crate::models::json::{JsonBuiltinMethod, JsonClassMethod, JsonUtilityFunction};
use crate::special_cases::codegen_special_cases;
use crate::Context;

// Deliberately private -- all checks must go through `special_cases`.

#[rustfmt::skip]
pub fn is_class_method_deleted(class_name: &TyName, method: &JsonClassMethod, ctx: &mut Context) -> bool {
    if codegen_special_cases::is_class_method_excluded(method, ctx){
        return true;
    }
    
    match (class_name.godot_ty.as_str(), method.name.as_str()) {
        // Already covered by manual APIs.
        //| ("Object", "to_string")
        | ("Object", "get_instance_id")
        
        // Removed because it is a worse version of Node::get_node_or_null(): it _seems_ like it's fallible due to Option<T> return type,
        // however Godot will emit an error message if the node is absent. In the future with non-null types, this may be re-introduced.
        // Alternatively, both get_node/get_node_or_null could become generic and use the get_node_as/try_get_node_as impl (removing those).
        | ("Node", "get_node")

        // Removed in https://github.com/godotengine/godot/pull/88418, but they cannot reasonably be used before, either.
        | ("GDExtension", "open_library")
        | ("GDExtension", "initialize_library")
        | ("GDExtension", "close_library")

        // Thread APIs
        | ("ResourceLoader", "load_threaded_get")
        | ("ResourceLoader", "load_threaded_get_status")
        | ("ResourceLoader", "load_threaded_request")
        // also: enum ThreadLoadStatus

        // TODO: Godot exposed methods that are unavailable, bug reported in https://github.com/godotengine/godot/issues/90303.
        | ("OpenXRHand", "set_hand_skeleton")
        | ("OpenXRHand", "get_hand_skeleton")
        | ("SkeletonIK3D", "set_interpolation")
        | ("SkeletonIK3D", "get_interpolation")
        | ("VisualShaderNodeComment", "set_title")
        | ("VisualShaderNodeComment", "get_title")
        | ("VisualShaderNodeComment", "set_description")
        | ("VisualShaderNodeComment", "get_description")

        => true, _ => false
    }
}

pub fn is_class_deleted(class_name: &TyName) -> bool {
    codegen_special_cases::is_class_excluded(&class_name.godot_ty)
        || is_godot_type_deleted(&class_name.godot_ty)
}

pub fn is_godot_type_deleted(godot_ty: &str) -> bool {
    // Note: parameter can be a class or builtin name, but also something like "enum::AESContext.Mode".

    // Exclude experimental APIs unless opted-in.
    if !cfg!(feature = "experimental-godot-api") && is_class_experimental(godot_ty) {
        return true;
    }

    // OpenXR has not been available for macOS before 4.2.
    // See e.g. https://github.com/GodotVR/godot-xr-tools/issues/479.
    // OpenXR is also not available on iOS: https://github.com/godotengine/godot/blob/13ba673c42951fd7cfa6fd8a7f25ede7e9ad92bb/modules/openxr/config.py#L2
    // Do not hardcode a list of OpenXR classes, as more may be added in future Godot versions; instead use prefix.
    if godot_ty.starts_with("OpenXR") {
        let target_os = std::env::var("CARGO_CFG_TARGET_OS");
        match target_os.as_deref() {
            Ok("ios") => return true,
            Ok("macos") => {
                #[cfg(before_api = "4.2")]
                return true;
            }
            _ => {}
        }
    }

    match godot_ty {
        // Hardcoded cases that are not accessible.
        // Only on Android.
        | "JavaClassWrapper"
        | "JNISingleton"
        | "JavaClass"
        // Only on WASM.
        | "JavaScriptBridge"
        | "JavaScriptObject"

        // Thread APIs.
        | "Thread"
        | "Mutex"
        | "Semaphore"

        // Internal classes that were removed in https://github.com/godotengine/godot/pull/80852, but are still available for API < 4.2.
        | "FramebufferCacheRD"
        | "GDScriptEditorTranslationParserPlugin"
        | "GDScriptNativeClass"
        | "GLTFDocumentExtensionPhysics"
        | "GLTFDocumentExtensionTextureWebP"
        | "GodotPhysicsServer2D"
        | "GodotPhysicsServer3D"
        | "IPUnix"
        | "MovieWriterMJPEG"
        | "MovieWriterPNGWAV"
        | "ResourceFormatImporterSaver"
        => true,
        // Previously loaded lazily; in 4.2 it loads at the Scene level. See: https://github.com/godotengine/godot/pull/81305
        | "ThemeDB"
        => cfg!(before_api = "4.2"),
        // reintroduced in 4.3. See: https://github.com/godotengine/godot/pull/80214
        | "UniformSetCacheRD"
        => cfg!(before_api = "4.3"),
        _ => false
    }
}

#[rustfmt::skip]
pub fn is_class_experimental(godot_class_name: &str) -> bool {
    // Note: parameter can be a class or builtin name, but also something like "enum::AESContext.Mode".

    // These classes are currently hardcoded, but the information is available in Godot's doc/classes directory.
    // The XML file contains a property <class name="NavigationMesh" ... is_experimental="true">.

    match godot_class_name {
        | "GraphEdit"
        | "GraphNode"
        | "NavigationAgent2D"
        | "NavigationAgent3D"
        | "NavigationLink2D"
        | "NavigationLink3D"
        | "NavigationMesh"
        | "NavigationMeshSourceGeometryData3D"
        | "NavigationObstacle2D"
        | "NavigationObstacle3D"
        | "NavigationPathQueryParameters2D"
        | "NavigationPathQueryParameters3D"
        | "NavigationPathQueryResult2D"
        | "NavigationPathQueryResult3D"
        | "NavigationPolygon"
        | "NavigationRegion2D"
        | "NavigationRegion3D"
        | "NavigationServer2D"
        | "NavigationServer3D"
        | "SkeletonModification2D"
        | "SkeletonModification2DCCDIK"
        | "SkeletonModification2DFABRIK"
        | "SkeletonModification2DJiggle"
        | "SkeletonModification2DLookAt"
        | "SkeletonModification2DPhysicalBones"
        | "SkeletonModification2DStackHolder"
        | "SkeletonModification2DTwoBoneIK"
        | "SkeletonModificationStack2D"
        | "StreamPeerGZIP"
        | "TextureRect"
        
        => true, _ => false
    }
}

/// Whether a method is available in the method table as a named accessor.
#[rustfmt::skip]
pub fn is_named_accessor_in_table(class_or_builtin_ty: &TyName, godot_method_name: &str) -> bool {
    // Hand-selected APIs.
    match (class_or_builtin_ty.godot_ty.as_str(), godot_method_name) {
        | ("OS", "has_feature")

        => return true, _ => {}
    }

    // Generated methods made private are typically needed internally and exposed with a different API,
    // so make them accessible.
    is_method_private(class_or_builtin_ty, godot_method_name)
}

/// Whether a class or builtin method should be hidden from the public API.
#[rustfmt::skip]
pub fn is_method_private(class_or_builtin_ty: &TyName, godot_method_name: &str) -> bool {
    match (class_or_builtin_ty.godot_ty.as_str(), godot_method_name) {
        // Already covered by manual APIs
        | ("Object", "to_string")
        | ("RefCounted", "init_ref")
        | ("RefCounted", "reference")
        | ("RefCounted", "unreference")
        | ("Object", "notification")

        => true, _ => false
    }
}

#[rustfmt::skip]
pub fn is_method_excluded_from_default_params(class_name: Option<&TyName>, godot_method_name: &str) -> bool {
    // None if global/utilities function
    let class_name = class_name.map_or("", |ty| ty.godot_ty.as_str());

    match (class_name, godot_method_name) {
        | ("Object", "notification")

        => true, _ => false
    }
}

/// Return `true` if a method should have `&self` receiver in Rust, `false` if `&mut self` and `None` if original qualifier should be kept.
///
/// In cases where the method falls under some general category (like getters) that have their own const-qualification overrides, `Some`
/// should be returned to take precedence over general rules. Example: `FileAccess::get_pascal_string()` is mut, but would be const-qualified
/// since it looks like a getter.
#[rustfmt::skip]
pub fn is_class_method_const(class_name: &TyName, godot_method: &JsonClassMethod) -> Option<bool> {
    match (class_name.godot_ty.as_str(), godot_method.name.as_str()) {
        // Changed to const.
        | ("Object", "to_string")
        => Some(true),

        // Changed to mut.
        // Needs some fixes to make sure _ex() builders have consistent signature, e.g. FileAccess::get_csv_line_full().
        /*
        | ("FileAccess", "get_16")
        | ("FileAccess", "get_32")
        | ("FileAccess", "get_64")
        | ("FileAccess", "get_8")
        | ("FileAccess", "get_csv_line")
        | ("FileAccess", "get_real")
        | ("FileAccess", "get_float")
        | ("FileAccess", "get_double")
        | ("FileAccess", "get_var")
        | ("FileAccess", "get_line")
        | ("FileAccess", "get_pascal_string") // already mut.
        | ("StreamPeer", "get_8")
        | ("StreamPeer", "get_16")
        | ("StreamPeer", "get_32")
        | ("StreamPeer", "get_64")
        | ("StreamPeer", "get_float")
        | ("StreamPeer", "get_double")
        => Some(false),
        */
        
        _ => {
            // TODO Many getters are mutably qualified (GltfAccessor::get_max, CameraAttributes::get_exposure_multiplier, ...).
            // As a default, set those to const.

            None
        },
    }
}

/// True if builtin method is excluded. Does NOT check for type exclusion; use [`is_builtin_type_deleted`] for that.
pub fn is_builtin_method_deleted(_class_name: &TyName, method: &JsonBuiltinMethod) -> bool {
    // Currently only deleted if codegen.
    codegen_special_cases::is_builtin_method_excluded(method)
}

/// True if builtin type is excluded (`NIL` or scalars)
pub fn is_builtin_type_deleted(class_name: &TyName) -> bool {
    let name = class_name.godot_ty.as_str();
    name == "Nil" || is_builtin_type_scalar(name)
}

/// True if `int`, `float`, `bool`, ...
pub fn is_builtin_type_scalar(name: &str) -> bool {
    name.chars().next().unwrap().is_ascii_lowercase()
}

#[rustfmt::skip]
pub fn is_utility_function_deleted(function: &JsonUtilityFunction, ctx: &mut Context) -> bool {
    /*let hardcoded = match function.name.as_str() {
        | "..."

        => true, _ => false
    };

    hardcoded ||*/ codegen_special_cases::is_utility_function_excluded(function, ctx)
}

pub fn maybe_rename_class_method<'m>(class_name: &TyName, godot_method_name: &'m str) -> &'m str {
    match (class_name.godot_ty.as_str(), godot_method_name) {
        // GDScript, GDScriptNativeClass, possibly more in the future
        (_, "new") => "instantiate",
        _ => godot_method_name,
    }
}

// Maybe merge with above?
pub fn maybe_rename_virtual_method(rust_method_name: &str) -> &str {
    // A few classes define a virtual method called "_init" (distinct from the constructor)
    // -> rename those to avoid a name conflict in I* interface trait.
    match rust_method_name {
        "init" => "init_ext",
        _ => rust_method_name,
    }
}

pub fn get_class_extra_docs(class_name: &TyName) -> Option<&'static str> {
    match class_name.godot_ty.as_str() {
        "FileAccess" => {
            Some("The gdext library provides a higher-level abstraction, which should be preferred: [`GFile`][crate::tools::GFile].")
        }
        "ScriptExtension" => {
            Some("Use this in combination with the [`obj::script` module][crate::obj::script].")
        }

        _ => None,
    }
}

pub fn get_interface_extra_docs(trait_name: &str) -> Option<&'static str> {
    match trait_name {
        "IScriptExtension" => {
            Some("Use this in combination with the [`obj::script` module][crate::obj::script].")
        }

        _ => None,
    }
}

#[rustfmt::skip]
pub fn is_class_level_server(class_name: &str) -> bool {
    // Unclear on if some of these classes should be registered earlier than `Scene`:
    // - `RenderData` + `RenderDataExtension`
    // - `RenderSceneData` + `RenderSceneDataExtension`

    match class_name {
        // TODO: These should actually be at level `Core`
        | "Object" | "OpenXRExtensionWrapperExtension" 

        // Shouldn't be inherited from in rust but are still servers.
        | "AudioServer" | "CameraServer" | "NavigationServer2D" | "NavigationServer3D" | "RenderingServer" | "TranslationServer" | "XRServer" 

        // PhysicsServer2D
        | "PhysicsDirectBodyState2D" | "PhysicsDirectBodyState2DExtension" 
        | "PhysicsDirectSpaceState2D" | "PhysicsDirectSpaceState2DExtension" 
        | "PhysicsServer2D" | "PhysicsServer2DExtension" 
        | "PhysicsServer2DManager" 

        // PhysicsServer3D
        | "PhysicsDirectBodyState3D" | "PhysicsDirectBodyState3DExtension" 
        | "PhysicsDirectSpaceState3D" | "PhysicsDirectSpaceState3DExtension" 
        | "PhysicsServer3D" | "PhysicsServer3DExtension" 
        | "PhysicsServer3DManager" 
        | "PhysicsServer3DRenderingServerHandler"

        => true, _ => false
    }
}

/// Whether a generated enum is `pub(crate)`; useful for manual re-exports.
#[rustfmt::skip]
pub fn is_enum_private(class_name: Option<&TyName>, enum_name: &str) -> bool {
    match (class_name, enum_name) {
        // Re-exported to godot::builtin.
        | (None, "Corner")
        | (None, "EulerOrder")
        | (None, "Side")
        | (None, "Variant.Operator")
        | (None, "Variant.Type")

        => true, _ => false
    }
}

/// Certain enums that are extremely unlikely to get new identifiers in the future.
/// 
/// `class_name` = None for global enums.
/// 
/// Very conservative, only includes a few enums. Even `VariantType` was extended over time.
/// Also does not work for any enums containing duplicate ordinals.
#[rustfmt::skip]
pub fn is_enum_exhaustive(class_name: Option<&TyName>, enum_name: &str) -> bool {
    // Adding new enums here should generally not break existing code:
    // * match _ patterns are still allowed, but cause a warning
    // * Enum::CONSTANT access looks the same for proper enum and newtype+const
    // Obviously, removing them will.

    match (class_name, enum_name) {
        | (None, "ClockDirection")
        | (None, "Corner")
        | (None, "EulerOrder")
        | (None, "Side")
        | (None, "Orientation")

        => true, _ => false
    }
}
