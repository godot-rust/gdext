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

use crate::conv::to_enum_type_uncached;
use crate::models::domain::{Enum, RustTy, TyName};
use crate::models::json::{JsonBuiltinMethod, JsonClassMethod, JsonUtilityFunction};
use crate::special_cases::codegen_special_cases;
use crate::Context;
use proc_macro2::Ident;
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

        // TODO: Godot exposed methods that are unavailable, bug reported in https://github.com/godotengine/godot/issues/90303.
        | ("OpenXRHand", "set_hand_skeleton")
        | ("OpenXRHand", "get_hand_skeleton")
        | ("SkeletonIK3D", "set_interpolation")
        | ("SkeletonIK3D", "get_interpolation")
        | ("VisualShaderNodeComment", "set_title")
        | ("VisualShaderNodeComment", "get_title")
        | ("VisualShaderNodeComment", "set_description")
        | ("VisualShaderNodeComment", "get_description")
        => true,
        
        // Workaround for methods unexposed in Release mode, see https://github.com/godotengine/godot/pull/100317
        // and https://github.com/godotengine/godot/pull/100328.
        #[cfg(not(debug_assertions))]
        | ("CollisionShape2D", "set_debug_color")
        | ("CollisionShape2D", "get_debug_color")
        | ("CollisionShape3D", "set_debug_color")
        | ("CollisionShape3D", "get_debug_color")
        | ("CollisionShape3D", "set_debug_fill_enabled")
        | ("CollisionShape3D", "get_debug_fill_enabled") => true,

        // Thread APIs
        #[cfg(not(feature = "experimental-threads"))]
        | ("ResourceLoader", "load_threaded_get")
        | ("ResourceLoader", "load_threaded_get_status")
        | ("ResourceLoader", "load_threaded_request") => true,
        // also: enum ThreadLoadStatus

        _ => false
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
    // OpenXR is also not available on iOS and Web: https://github.com/godotengine/godot/blob/13ba673c42951fd7cfa6fd8a7f25ede7e9ad92bb/modules/openxr/config.py#L2
    // Do not hardcode a list of OpenXR classes, as more may be added in future Godot versions; instead use prefix.
    if godot_ty.starts_with("OpenXR") {
        let target_os = std::env::var("CARGO_CFG_TARGET_OS");
        match target_os.as_deref() {
            Ok("ios") | Ok("emscripten") => return true,
            Ok("macos") => {
                #[cfg(before_api = "4.2")] #[cfg_attr(published_docs, doc(cfg(before_api = "4.2")))]
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
    // The XML file contains a property <class name="NavigationMesh" ... experimental="">.

    // Last update: 2024-09-15; Godot rev 6681f2563b99e14929a8acb27f4908fece398ef1.
    match godot_class_name {
        | "AudioSample"
        | "AudioSamplePlayback"
        | "Compositor"
        | "CompositorEffect"
        | "GraphEdit"
        | "GraphElement"
        | "GraphFrame"
        | "GraphNode"
        | "NavigationAgent2D"
        | "NavigationAgent3D"
        | "NavigationLink2D"
        | "NavigationLink3D"
        | "NavigationMesh"
        | "NavigationMeshSourceGeometryData2D"
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
        | "Parallax2D"
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
        | "XRBodyModifier3D"
        | "XRBodyTracker"
        | "XRFaceModifier3D"
        | "XRFaceTracker"

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
///
/// Builtin class methods are all private by default, due to being declared in an `Inner*` struct. A separate mechanism is used
/// to make them public, see [`is_builtin_method_exposed`].
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
pub fn is_builtin_method_exposed(builtin_ty: &TyName, godot_method_name: &str) -> bool {
    match (builtin_ty.godot_ty.as_str(), godot_method_name) {
        // GString
        | ("String", "begins_with")
        | ("String", "ends_with")
        | ("String", "is_subsequence_of")
        | ("String", "is_subsequence_ofn")
        | ("String", "bigrams")
        | ("String", "similarity")
        | ("String", "replace")
        | ("String", "replacen")
        | ("String", "repeat")
        | ("String", "reverse")
        | ("String", "capitalize")
        | ("String", "to_camel_case")
        | ("String", "to_pascal_case")
        | ("String", "to_snake_case")
        | ("String", "split_floats")
        | ("String", "join")
        | ("String", "to_upper")
        | ("String", "to_lower")
        | ("String", "left")
        | ("String", "right")
        | ("String", "strip_edges")
        | ("String", "strip_escapes")
        | ("String", "lstrip")
        | ("String", "rstrip")
        | ("String", "get_extension")
        | ("String", "get_basename")
        | ("String", "path_join")
        | ("String", "indent")
        | ("String", "dedent")
        | ("String", "md5_text")
        | ("String", "sha1_text")
        | ("String", "sha256_text")
        | ("String", "md5_buffer")
        | ("String", "sha1_buffer")
        | ("String", "sha256_buffer")
        | ("String", "is_empty")
        | ("String", "contains")
        | ("String", "containsn")
        | ("String", "is_absolute_path")
        | ("String", "is_relative_path")
        | ("String", "simplify_path")
        | ("String", "get_base_dir")
        | ("String", "get_file")
        | ("String", "xml_escape")
        | ("String", "xml_unescape")
        | ("String", "uri_encode")
        | ("String", "uri_decode")
        | ("String", "c_escape")
        | ("String", "c_unescape")
        | ("String", "json_escape")
        | ("String", "validate_node_name")
        | ("String", "validate_filename")
        | ("String", "is_valid_identifier")
        | ("String", "is_valid_int")
        | ("String", "is_valid_float")
        | ("String", "is_valid_hex_number")
        | ("String", "is_valid_html_color")
        | ("String", "is_valid_ip_address")
        | ("String", "is_valid_filename")
        | ("String", "to_int")
        | ("String", "to_float")
        | ("String", "hex_to_int")
        | ("String", "bin_to_int")
        | ("String", "trim_prefix")
        | ("String", "trim_suffix")
        | ("String", "to_ascii_buffer")
        | ("String", "to_utf8_buffer")
        | ("String", "to_utf16_buffer")
        | ("String", "to_utf32_buffer")
        | ("String", "hex_decode")
        | ("String", "to_wchar_buffer")
        | ("String", "num_scientific")
        | ("String", "num")
        | ("String", "num_int64")
        | ("String", "num_uint64")
        | ("String", "chr")
        | ("String", "humanize_size")

        // StringName
        | ("StringName", "begins_with")
        | ("StringName", "ends_with")
        | ("StringName", "is_subsequence_of")
        | ("StringName", "is_subsequence_ofn")
        | ("StringName", "bigrams")
        | ("StringName", "similarity")
        | ("StringName", "replace")
        | ("StringName", "replacen")
        | ("StringName", "repeat")
        | ("StringName", "reverse")
        | ("StringName", "capitalize")
        | ("StringName", "to_camel_case")
        | ("StringName", "to_pascal_case")
        | ("StringName", "to_snake_case")
        | ("StringName", "split_floats")
        | ("StringName", "join")
        | ("StringName", "to_upper")
        | ("StringName", "to_lower")
        | ("StringName", "left")
        | ("StringName", "right")
        | ("StringName", "strip_edges")
        | ("StringName", "strip_escapes")
        | ("StringName", "lstrip")
        | ("StringName", "rstrip")
        | ("StringName", "get_extension")
        | ("StringName", "get_basename")
        | ("StringName", "path_join")
        | ("StringName", "indent")
        | ("StringName", "dedent")
        | ("StringName", "md5_text")
        | ("StringName", "sha1_text")
        | ("StringName", "sha256_text")
        | ("StringName", "md5_buffer")
        | ("StringName", "sha1_buffer")
        | ("StringName", "sha256_buffer")
        | ("StringName", "is_empty")
        | ("StringName", "contains")
        | ("StringName", "containsn")
        | ("StringName", "is_absolute_path")
        | ("StringName", "is_relative_path")
        | ("StringName", "simplify_path")
        | ("StringName", "get_base_dir")
        | ("StringName", "get_file")
        | ("StringName", "xml_escape")
        | ("StringName", "xml_unescape")
        | ("StringName", "uri_encode")
        | ("StringName", "uri_decode")
        | ("StringName", "c_escape")
        | ("StringName", "c_unescape")
        | ("StringName", "json_escape")
        | ("StringName", "validate_node_name")
        | ("StringName", "validate_filename")
        | ("StringName", "is_valid_identifier")
        | ("StringName", "is_valid_int")
        | ("StringName", "is_valid_float")
        | ("StringName", "is_valid_hex_number")
        | ("StringName", "is_valid_html_color")
        | ("StringName", "is_valid_ip_address")
        | ("StringName", "is_valid_filename")
        | ("StringName", "to_int")
        | ("StringName", "to_float")
        | ("StringName", "hex_to_int")
        | ("StringName", "bin_to_int")
        | ("StringName", "trim_prefix")
        | ("StringName", "trim_suffix")
        | ("StringName", "to_ascii_buffer")
        | ("StringName", "to_utf8_buffer")
        | ("StringName", "to_utf16_buffer")
        | ("StringName", "to_utf32_buffer")
        | ("StringName", "hex_decode")
        | ("StringName", "to_wchar_buffer")

        // NodePath
        | ("NodePath", "is_absolute")
        | ("NodePath", "is_empty")
        | ("NodePath", "get_concatenated_names")
        | ("NodePath", "get_concatenated_subnames")
        | ("NodePath", "get_as_property_path")

        // Callable
        | ("Callable", "call")
        | ("Callable", "call_deferred")
        | ("Callable", "bind")
        | ("Callable", "get_bound_arguments")
        | ("Callable", "rpc")
        | ("Callable", "rpc_id")

        // PackedByteArray
        | ("PackedByteArray", "get_string_from_ascii")
        | ("PackedByteArray", "get_string_from_utf8")
        | ("PackedByteArray", "get_string_from_utf16")
        | ("PackedByteArray", "get_string_from_utf32")
        | ("PackedByteArray", "get_string_from_wchar")
        | ("PackedByteArray", "hex_encode")

        // Vector2i
        | ("Vector2i", "clampi")
        | ("Vector2i", "distance_squared_to")
        | ("Vector2i", "distance_to")
        | ("Vector2i", "maxi")
        | ("Vector2i", "mini")
        | ("Vector2i", "snappedi")

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

/// Currently only for virtual methods; checks if the specified parameter is required (non-null) and can be declared as `Gd<T>`
/// instead of `Option<Gd<T>>`.
pub fn is_class_method_param_required(
    class_name: &TyName,
    method_name: &str,
    param: &Ident, // Don't use `&str` to avoid to_string() allocations for each check on call-site.
) -> bool {
    // Note: magically, it's enough if a base class method is declared here; it will be picked up by derived classes.

    match (class_name.godot_ty.as_str(), method_name) {
        // Nodes.
        ("Node", "input") => true,
        ("Node", "shortcut_input") => true,
        ("Node", "unhandled_input") => true,
        ("Node", "unhandled_key_input") => true,

        // https://docs.godotengine.org/en/stable/classes/class_collisionobject2d.html#class-collisionobject2d-private-method-input-event
        ("CollisionObject2D", "input_event") => true, // both parameters.

        // UI.
        ("Control", "gui_input") => true,

        // Script instances.
        ("ScriptExtension", "instance_create") => param == "for_object",
        ("ScriptExtension", "placeholder_instance_create") => param == "for_object",
        ("ScriptExtension", "inherits_script") => param == "script",
        ("ScriptExtension", "instance_has") => param == "object",

        // Editor.
        ("EditorExportPlugin", "customize_resource") => param == "resource",
        ("EditorExportPlugin", "customize_scene") => param == "scene",

        ("EditorPlugin", "handles") => param == "object",

        _ => false,
    }
}

/// True if builtin method is excluded. Does NOT check for type exclusion; use [`is_builtin_type_deleted`] for that.
pub fn is_builtin_method_deleted(_class_name: &TyName, method: &JsonBuiltinMethod) -> bool {
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
    // This is for non-virtual methods only. For virtual methods, use other handler below.

    match (class_name.godot_ty.as_str(), godot_method_name) {
        // GDScript, GDScriptNativeClass, possibly more in the future
        (_, "new") => "instantiate",

        _ => godot_method_name,
    }
}

// Maybe merge with above?
pub fn maybe_rename_virtual_method<'m>(class_name: &TyName, rust_method_name: &'m str) -> &'m str {
    match (class_name.godot_ty.as_str(), rust_method_name) {
        // Workaround for 2 methods of same name; see https://github.com/godotengine/godot/pull/99181#issuecomment-2543311415.
        ("AnimationNodeExtension", "process") => "process_animation",

        // A few classes define a virtual method called "_init" (distinct from the constructor)
        // -> rename those to avoid a name conflict in I* interface trait.
        (_, "init") => "init_ext",

        _ => rust_method_name,
    }
}

// TODO method-level extra docs, for:
// - Node::rpc_config() -> link to RpcConfig.

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

#[cfg(before_api = "4.4")] #[cfg_attr(published_docs, doc(cfg(before_api = "4.4")))]
pub fn is_virtual_method_required(class_name: &str, method: &str) -> bool {
    match (class_name, method) {
        ("ScriptLanguageExtension", _) => method != "get_doc_comment_delimiters",

        ("ScriptExtension", "editor_can_reload_from_file")
        | ("ScriptExtension", "can_instantiate")
        | ("ScriptExtension", "get_base_script")
        | ("ScriptExtension", "get_global_name")
        | ("ScriptExtension", "inherits_script")
        | ("ScriptExtension", "get_instance_base_type")
        | ("ScriptExtension", "instance_create")
        | ("ScriptExtension", "placeholder_instance_create")
        | ("ScriptExtension", "instance_has")
        | ("ScriptExtension", "has_source_code")
        | ("ScriptExtension", "get_source_code")
        | ("ScriptExtension", "set_source_code")
        | ("ScriptExtension", "reload")
        | ("ScriptExtension", "get_documentation")
        | ("ScriptExtension", "has_method")
        | ("ScriptExtension", "has_static_method")
        | ("ScriptExtension", "get_method_info")
        | ("ScriptExtension", "is_tool")
        | ("ScriptExtension", "is_valid")
        | ("ScriptExtension", "get_language")
        | ("ScriptExtension", "has_script_signal")
        | ("ScriptExtension", "get_script_signal_list")
        | ("ScriptExtension", "has_property_default_value")
        | ("ScriptExtension", "get_property_default_value")
        | ("ScriptExtension", "update_exports")
        | ("ScriptExtension", "get_script_method_list")
        | ("ScriptExtension", "get_script_property_list")
        | ("ScriptExtension", "get_member_line")
        | ("ScriptExtension", "get_constants")
        | ("ScriptExtension", "get_members")
        | ("ScriptExtension", "is_placeholder_fallback_enabled")
        | ("ScriptExtension", "get_rpc_config")
        | ("EditorExportPlugin", "customize_resource")
        | ("EditorExportPlugin", "customize_scene")
        | ("EditorExportPlugin", "get_customization_configuration_hash")
        | ("EditorExportPlugin", "get_name")
        | ("EditorVcsInterface", _)
        | ("MovieWriter", _)
        | ("TextServerExtension", "has_feature")
        | ("TextServerExtension", "get_name")
        | ("TextServerExtension", "get_features")
        | ("TextServerExtension", "free_rid")
        | ("TextServerExtension", "has")
        | ("TextServerExtension", "create_font")
        | ("TextServerExtension", "font_set_fixed_size")
        | ("TextServerExtension", "font_get_fixed_size")
        | ("TextServerExtension", "font_set_fixed_size_scale_mode")
        | ("TextServerExtension", "font_get_fixed_size_scale_mode")
        | ("TextServerExtension", "font_get_size_cache_list")
        | ("TextServerExtension", "font_clear_size_cache")
        | ("TextServerExtension", "font_remove_size_cache")
        | ("TextServerExtension", "font_set_ascent")
        | ("TextServerExtension", "font_get_ascent")
        | ("TextServerExtension", "font_set_descent")
        | ("TextServerExtension", "font_get_descent")
        | ("TextServerExtension", "font_set_underline_position")
        | ("TextServerExtension", "font_get_underline_position")
        | ("TextServerExtension", "font_set_underline_thickness")
        | ("TextServerExtension", "font_get_underline_thickness")
        | ("TextServerExtension", "font_set_scale")
        | ("TextServerExtension", "font_get_scale")
        | ("TextServerExtension", "font_get_texture_count")
        | ("TextServerExtension", "font_clear_textures")
        | ("TextServerExtension", "font_remove_texture")
        | ("TextServerExtension", "font_set_texture_image")
        | ("TextServerExtension", "font_get_texture_image")
        | ("TextServerExtension", "font_get_glyph_list")
        | ("TextServerExtension", "font_clear_glyphs")
        | ("TextServerExtension", "font_remove_glyph")
        | ("TextServerExtension", "font_get_glyph_advance")
        | ("TextServerExtension", "font_set_glyph_advance")
        | ("TextServerExtension", "font_get_glyph_offset")
        | ("TextServerExtension", "font_set_glyph_offset")
        | ("TextServerExtension", "font_get_glyph_size")
        | ("TextServerExtension", "font_set_glyph_size")
        | ("TextServerExtension", "font_get_glyph_uv_rect")
        | ("TextServerExtension", "font_set_glyph_uv_rect")
        | ("TextServerExtension", "font_get_glyph_texture_idx")
        | ("TextServerExtension", "font_set_glyph_texture_idx")
        | ("TextServerExtension", "font_get_glyph_texture_rid")
        | ("TextServerExtension", "font_get_glyph_texture_size")
        | ("TextServerExtension", "font_get_glyph_index")
        | ("TextServerExtension", "font_get_char_from_glyph_index")
        | ("TextServerExtension", "font_has_char")
        | ("TextServerExtension", "font_get_supported_chars")
        | ("TextServerExtension", "font_draw_glyph")
        | ("TextServerExtension", "font_draw_glyph_outline")
        | ("TextServerExtension", "create_shaped_text")
        | ("TextServerExtension", "shaped_text_clear")
        | ("TextServerExtension", "shaped_text_add_string")
        | ("TextServerExtension", "shaped_text_add_object")
        | ("TextServerExtension", "shaped_text_resize_object")
        | ("TextServerExtension", "shaped_get_span_count")
        | ("TextServerExtension", "shaped_get_span_meta")
        | ("TextServerExtension", "shaped_set_span_update_font")
        | ("TextServerExtension", "shaped_text_substr")
        | ("TextServerExtension", "shaped_text_get_parent")
        | ("TextServerExtension", "shaped_text_shape")
        | ("TextServerExtension", "shaped_text_is_ready")
        | ("TextServerExtension", "shaped_text_get_glyphs")
        | ("TextServerExtension", "shaped_text_sort_logical")
        | ("TextServerExtension", "shaped_text_get_glyph_count")
        | ("TextServerExtension", "shaped_text_get_range")
        | ("TextServerExtension", "shaped_text_get_trim_pos")
        | ("TextServerExtension", "shaped_text_get_ellipsis_pos")
        | ("TextServerExtension", "shaped_text_get_ellipsis_glyphs")
        | ("TextServerExtension", "shaped_text_get_ellipsis_glyph_count")
        | ("TextServerExtension", "shaped_text_get_objects")
        | ("TextServerExtension", "shaped_text_get_object_rect")
        | ("TextServerExtension", "shaped_text_get_object_range")
        | ("TextServerExtension", "shaped_text_get_object_glyph")
        | ("TextServerExtension", "shaped_text_get_size")
        | ("TextServerExtension", "shaped_text_get_ascent")
        | ("TextServerExtension", "shaped_text_get_descent")
        | ("TextServerExtension", "shaped_text_get_width")
        | ("TextServerExtension", "shaped_text_get_underline_position")
        | ("TextServerExtension", "shaped_text_get_underline_thickness")
        | ("AudioStreamPlayback", "mix")
        | ("AudioStreamPlaybackResampled", "mix_resampled")
        | ("AudioStreamPlaybackResampled", "get_stream_sampling_rate")
        | ("AudioEffectInstance", "process")
        | ("AudioEffect", "instantiate")
        | ("PhysicsDirectBodyState2DExtension", _)
        | ("PhysicsDirectBodyState3DExtension", _)
        | ("PhysicsDirectSpaceState2DExtension", _)
        | ("PhysicsDirectSpaceState3DExtension", _)
        | ("PhysicsServer3DExtension", _)
        | ("PhysicsServer2DExtension", _)
        | ("EditorScript", "run")
        | ("VideoStreamPlayback", "update")
        | ("EditorFileSystemImportFormatSupportQuery", _)
        | ("Mesh", _)
        | ("Texture2D", "get_width")
        | ("Texture2D", "get_height")
        | ("Texture3D", _)
        | ("TextureLayered", _)
        | ("StyleBox", "draw")
        | ("Material", "get_shader_rid")
        | ("Material", "get_shader_mode")
        | ("PacketPeerExtension", "get_available_packet_count")
        | ("PacketPeerExtension", "get_max_packet_size")
        | ("StreamPeerExtension", "get_available_bytes")
        | ("WebRtcDataChannelExtension", "poll")
        | ("WebRtcDataChannelExtension", "close")
        | ("WebRtcDataChannelExtension", "set_write_mode")
        | ("WebRtcDataChannelExtension", "get_write_mode")
        | ("WebRtcDataChannelExtension", "was_string_packet")
        | ("WebRtcDataChannelExtension", "get_ready_state")
        | ("WebRtcDataChannelExtension", "get_label")
        | ("WebRtcDataChannelExtension", "is_ordered")
        | ("WebRtcDataChannelExtension", "get_id")
        | ("WebRtcDataChannelExtension", "get_max_packet_life_time")
        | ("WebRtcDataChannelExtension", "get_max_retransmits")
        | ("WebRtcDataChannelExtension", "get_protocol")
        | ("WebRtcDataChannelExtension", "is_negotiated")
        | ("WebRtcDataChannelExtension", "get_buffered_amount")
        | ("WebRtcDataChannelExtension", "get_available_packet_count")
        | ("WebRtcDataChannelExtension", "get_max_packet_size")
        | ("WebRtcPeerConnectionExtension", _)
        | ("MultiplayerPeerExtension", "get_available_packet_count")
        | ("MultiplayerPeerExtension", "get_max_packet_size")
        | ("MultiplayerPeerExtension", "set_transfer_channel")
        | ("MultiplayerPeerExtension", "get_transfer_channel")
        | ("MultiplayerPeerExtension", "set_transfer_mode")
        | ("MultiplayerPeerExtension", "get_transfer_mode")
        | ("MultiplayerPeerExtension", "set_target_peer")
        | ("MultiplayerPeerExtension", "get_packet_peer")
        | ("MultiplayerPeerExtension", "get_packet_mode")
        | ("MultiplayerPeerExtension", "get_packet_channel")
        | ("MultiplayerPeerExtension", "is_server")
        | ("MultiplayerPeerExtension", "poll")
        | ("MultiplayerPeerExtension", "close")
        | ("MultiplayerPeerExtension", "disconnect_peer")
        | ("MultiplayerPeerExtension", "get_unique_id")
        | ("MultiplayerPeerExtension", "get_connection_status") => true,

        (_, _) => false,
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

/// Whether an enum can be combined with another enum (return value) for bitmasking purposes.
///
/// If multiple masks are ever necessary, this can be extended to return a slice instead of Option.
///
/// If a mapping is found, returns the corresponding `RustTy`.
pub fn as_enum_bitmaskable(enum_: &Enum) -> Option<RustTy> {
    let class_name = enum_.surrounding_class.as_ref();
    let class_name_str = class_name.map(|ty| ty.godot_ty.as_str());
    let enum_name = enum_.godot_name.as_str();

    let mapped = match (class_name_str, enum_name) {
        (None, "Key") => "KeyModifierMask",
        (None, "MouseButton") => "MouseButtonMask",

        // For class enums:
        // (Some("ThisClass"), "Enum") => "SomeClass.MaskedEnum"
        _ => return None,
    };

    // Exhaustive enums map to Rust `enum`, which cannot hold other values.
    // Code flow: this is as_enum_bitmaskable() is still called even if is_enum_exhaustive() previously returned true.
    assert!(
        !is_enum_exhaustive(class_name, enum_name),
        "Enum {enum_name} with bitmask mapping cannot be exhaustive"
    );

    let rust_ty = to_enum_type_uncached(mapped, true);
    Some(rust_ty)
}
