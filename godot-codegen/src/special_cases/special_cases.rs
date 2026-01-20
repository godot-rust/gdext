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

// NOTE: the methods are generally implemented on Godot types and method names (e.g. AABB, not Aabb).
// The Godot ones are stable and have a single source of truth; while Rust ones can be called at multiple times in the process (e.g. after
// initial preprocessing/name mangling).

// This file is deliberately private -- all checks must go through `special_cases`.

#![allow(clippy::match_like_matches_macro)] // if there is only one rule

use std::borrow::Cow;

use proc_macro2::Ident;

use crate::conv::to_enum_type_uncached;
use crate::models::domain::{
    ClassCodegenLevel, Enum, EnumReplacements, FnReturn, RustTy, TyName, VirtualMethodPresence,
};
use crate::models::json::{JsonBuiltinMethod, JsonClassMethod, JsonSignal, JsonUtilityFunction};
use crate::special_cases::codegen_special_cases;
use crate::util::option_as_slice;
use crate::Context;

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

        // Removed in https://github.com/godotengine/godot/pull/98566
        | ("VisualShader", "set_graph_offset")
        | ("VisualShader", "get_graph_offset")
        => true,

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

/// Native-struct types excluded in minimal codegen, because they hold codegen-excluded classes as fields.
pub fn is_native_struct_excluded(ty: &str) -> bool {
    codegen_special_cases::is_native_struct_excluded(ty)
}

/// Overrides the definition string for native structures, if they have incorrect definitions in the JSON.
#[rustfmt::skip]
pub fn get_native_struct_definition(struct_name: &str) -> Option<&'static str> {
    match struct_name {
        // Glyph struct definition was corrected in Godot 4.6 to include missing `span_index` field.
        // See https://github.com/godotengine/godot/pull/108369.
        // #[cfg(before_api = "4.6")] // TODO(v0.5): enable this once upstream PR is merged.
        "Glyph" => Some(
            "int start = -1;int end = -1;uint8_t count = 0;uint8_t repeat = 1;uint16_t flags = 0;float x_off = 0.f;float y_off = 0.f;\
            float advance = 0.f;RID font_rid;int font_size = 0;int32_t index = 0;int span_index = -1"
        ),

        _ => None,
    }
}

#[rustfmt::skip]
pub fn is_godot_type_deleted(godot_ty: &str) -> bool {
    // Note: parameter can be a class or builtin name, but also something like "enum::AESContext.Mode".

    // Exclude experimental APIs unless opted-in.
    if !cfg!(feature = "experimental-godot-api") && is_class_experimental(godot_ty) {
        return true;
    }

    // OpenXR has not been available for "macos" before 4.2 (now no longer supported by godot-rust).
    // See e.g. https://github.com/GodotVR/godot-xr-tools/issues/479.
    // OpenXR is also not available on iOS and Web: https://github.com/godotengine/godot/blob/13ba673c42951fd7cfa6fd8a7f25ede7e9ad92bb/modules/openxr/config.py#L2
    // Do not hardcode a list of OpenXR classes, as more may be added in future Godot versions; instead use prefix.
    if godot_ty.starts_with("OpenXR") {
        let target_os = std::env::var("CARGO_CFG_TARGET_OS");
        match target_os.as_deref() {
            Ok("ios") | Ok("emscripten") => return true,
            _ => {}
        }
    }

    // cfg!(target_os = "...") are relatively new and need more testing. If causing problems, revert to `true` (deleted) for now.
    // TODO(v0.5): for doc generation, consider moving the target-filters to the generated code, so that API docs still show the classes.
    match godot_ty {
        // Only on Android.
        | "JavaClass"
        | "JavaClassWrapper"
        | "JavaObject"
        | "JNISingleton"
        => !cfg!(target_os = "android"),

        // Only on Wasm.
        | "JavaScriptBridge"
        | "JavaScriptObject"
        => !cfg!(target_os = "emscripten"),

        // Thread APIs.
        | "Thread"
        | "Mutex"
        | "Semaphore"
        => true,

        // Reintroduced in 4.3: https://github.com/godotengine/godot/pull/80214
        | "UniformSetCacheRD"
        => cfg!(before_api = "4.3"),

        _ => false
    }

    // Older special cases:
    // * ThemeDB was loaded lazily; from 4.2 it loads at the Scene level: https://github.com/godotengine/godot/pull/81305
    // * Internal classes were accidentally exposed < 4.2: https://github.com/godotengine/godot/pull/80852: FramebufferCacheRD,
    //   GDScriptEditorTranslationParserPlugin, GDScriptNativeClass, GLTFDocumentExtensionPhysics, GLTFDocumentExtensionTextureWebP,
    //   GodotPhysicsServer2D, GodotPhysicsServer3D, IPUnix, MovieWriterMJPEG, MovieWriterPNGWAV, ResourceFormatImporterSaver
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

/// Whether a class can be instantiated (overrides Godot's defaults in some cases).
///
/// Returns `None` if the Godot default should be taken.
pub fn is_class_instantiable(class_ty: &TyName) -> Option<bool> {
    let class_name = class_ty.godot_ty.as_str();

    // The default constructor is available but callers meet with the following Godot error:
    // "ERROR: XY can't be created directly. Use create_tween() method."
    // for the following classes XY:
    //Tween, PropertyTweener, PropertyTweener, IntervalTweener, CallbackTweener, MethodTweener, SubtweenTweener,

    if class_name == "Tween" || class_name.ends_with("Tweener") {
        return Some(false);
    }

    None
}

/// Whether a class is "Godot abstract".
///
/// Abstract in Godot is different from the usual term in OOP. It means:
/// 1. The class has no default constructor. However, it can still be instantiated through other means; e.g. FileAccess::open().
/// 2. The class can not be inherited from *outside the engine*. It's possible for other engine classes to inherit from it, but not extension ones.
#[rustfmt::skip]
pub fn is_class_abstract(class_ty: &TyName) -> bool {
    // Get this list by running following command in Godot repo:
    // rg GDREGISTER_ABSTRACT_CLASS | rg -v '#define' | sd '.+\((\w+)\).+' '| "$1"' | sort | uniq > abstract.txt

    // Note: singletons are currently not declared abstract in Godot, but they are separately considered for the "final" property.

    match class_ty.godot_ty.as_str() {
        | "AnimationMixer"
        | "AudioEffectSpectrumAnalyzerInstance"
        | "AudioStreamGeneratorPlayback"
        | "AudioStreamPlaybackInteractive"
        | "AudioStreamPlaybackPlaylist"
        | "AudioStreamPlaybackPolyphonic"
        | "AudioStreamPlaybackSynchronized"
        | "BaseMaterial3D"
        | "CanvasItem"
        | "CollisionObject2D"
        | "CollisionObject3D"
        | "CompressedTextureLayered"
        | "CSGPrimitive3D"
        | "CSGShape3D"
        | "DirAccess"
        | "DisplayServer"
        | "EditorDebuggerSession"
        | "EditorExportPlatform"
        | "EditorExportPlatformAppleEmbedded"
        | "EditorExportPlatformPC"
        | "EditorExportPreset"
        | "EditorFileSystem"
        | "EditorInterface"
        | "EditorResourcePreview"
        | "EditorToaster"
        | "EditorUndoRedoManager"
        | "ENetPacketPeer"
        | "FileAccess"
        | "FileSystemDock"
        | "Font"
        | "GDExtensionManager"
        | "GPUParticlesAttractor3D"
        | "GPUParticlesCollision3D"
        | "ImageFormatLoader"
        | "ImageTextureLayered"
        | "Input"
        | "InputEvent"
        | "InputEventFromWindow"
        | "InputEventGesture"
        | "InputEventMouse"
        | "InputEventWithModifiers"
        | "InstancePlaceholder"
        | "IP"
        | "JavaScriptBridge"
        | "JavaScriptObject"
        | "Joint2D"
        | "Joint3D"
        | "Light2D"
        | "Light3D"
        | "Lightmapper"
        | "MultiplayerAPI"
        | "MultiplayerPeer"
        | "NavigationServer2D"
        | "NavigationServer3D"
        | "Node3DGizmo"
        | "Noise"
        | "Occluder3D"
        | "OpenXRBindingModifier"
        | "OpenXRCompositionLayer"
        | "OpenXRFutureResult"
        | "OpenXRHapticBase"
        | "OpenXRInteractionProfileEditorBase"
        | "PackedDataContainerRef"
        | "PacketPeer"
        | "PhysicsBody2D"
        | "PhysicsBody3D"
        | "PhysicsDirectBodyState2D"
        | "PhysicsDirectBodyState3D"
        | "PhysicsDirectSpaceState2D"
        | "PhysicsDirectSpaceState3D"
        | "PhysicsServer2D"
        | "PhysicsServer3D"
        | "PlaceholderTextureLayered"
        | "RenderData"
        | "RenderingDevice"
        | "RenderingServer"
        | "RenderSceneBuffers"
        | "RenderSceneData"
        | "ResourceImporter"
        | "ResourceUID"
        | "SceneState"
        | "SceneTreeTimer"
        | "Script"
        | "ScriptEditor"
        | "ScriptEditorBase"
        | "ScriptLanguage"
        | "ScrollBar"
        | "Separator"
        | "Shader"
        | "Shape2D"
        | "Shape3D"
        | "SkinReference"
        | "Slider"
        | "SpriteBase3D"
        | "StreamPeer"
        | "TextServer"
        | "TextureLayeredRD"
        | "TileSetSource"
        | "TLSOptions"
        | "TreeItem"
        | "Tweener"
        | "Viewport"
        | "VisualShaderNode"
        | "VisualShaderNodeConstant"
        | "VisualShaderNodeGroupBase"
        | "VisualShaderNodeOutput"
        | "VisualShaderNodeParameter"
        | "VisualShaderNodeParticleEmitter"
        | "VisualShaderNodeResizableBase"
        | "VisualShaderNodeSample3D"
        | "VisualShaderNodeTextureParameter"
        | "VisualShaderNodeVarying"
        | "VisualShaderNodeVectorBase"
        | "WebRTCDataChannel"
        | "WebXRInterface"
        | "WorkerThreadPool"
        | "XRInterface"
        | "XRTracker"
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
    if is_class_method_replaced_with_type_safe(class_or_builtin_ty, godot_method_name) {
        return true;
    }

    match (class_or_builtin_ty.godot_ty.as_str(), godot_method_name) {
        // Already covered by manual APIs
        | ("Object", "to_string")
        | ("Object", "notification")
        | ("RefCounted", "init_ref")
        | ("RefCounted", "reference")
        | ("RefCounted", "unreference")

        => true, _ => false
    }
}

/// Lists methods that are replaced with manual, more type-safe equivalents. See `type_safe_replacements.rs`.
///
/// See also [`get_class_method_enum_param_replacement()`] for a more automated approach specifically for enum parameters.
#[rustfmt::skip]
fn is_class_method_replaced_with_type_safe(class_ty: &TyName, godot_method_name: &str) -> bool {
    match (class_ty.godot_ty.as_str(), godot_method_name) {
        // Variant -> Option<Gd<Script>>
        | ("Object", "get_script")
        | ("Object", "set_script")

        // u32 -> ConnectFlags
        | ("Object", "connect")

        // i32 -> CallGroupFlags
        // Some of those (not the notifications) could be handled by automated enum replacement, but keeping them together is simpler.
        | ("SceneTree", "call_group_flags")
        | ("SceneTree", "notify_group")
        | ("SceneTree", "notify_group_flags")
        | ("SceneTree", "set_group_flags")

        => true, _ => false
    }
}

/// For a given class method, returns all integer parameters and return types that have a type-safe enum/bitfield replacement.
///
/// Returns a list of tuples `(param_name, enum_type, is_bitfield)`, for example `[("mode_flags", "FileAccess.ModeFlags", true)]`.
/// Use empty string `""` as `param_name` to denote return type replacements, for example `[("", "Tree.DropModeFlags", true)]`.
///
/// The caller should verify that the parameters exist and are in fact of integer type.
/// Type-unsafety like this is quite common in Godot and can be easily patched at the codegen level.
/// See also [`is_class_method_replaced_with_type_safe()`] for hand-picked overrides.
// #[rustfmt::skip]
pub fn get_class_method_param_enum_replacement(
    class_ty: &TyName,
    godot_method_name: &str,
) -> EnumReplacements {
    let godot_class_name = class_ty.godot_ty.as_str();

    // Notes on replacement mechanism:
    // 1. Design is deliberately (class, method) => [(param, enum)] instead of (class, method, param) => enum,
    //    because this will catch typos/renames in parameter names -- the call site can verify parameter existence.
    // 2. Bitfield is explicitly specified because Godot's API JSON also contains "enum::" or "bitfield::" prefixes as part of the type,
    //    and it would be annoying to resolve that information at the stage of domain mapping (depends on mapping of all enums).
    // 3. Empty string "" refers to the return type.
    // 4. Several "mask" type properties are not bitfields but indeed numeric (e.g. collision masks, light masks, ...).

    // IMPORTANT: double-check that enum/bitfield classification is correct, or override it with is_enum_bitfield() below.
    // Lots of Godot `is_bitfield` values are wrong.

    match (godot_class_name, godot_method_name) {
        ("CharFXTransform", "get_glyph_flags") => &[("", "TextServer.GraphemeFlag", true)],
        ("CharFXTransform", "set_glyph_flags") => {
            &[("glyph_flags", "TextServer.GraphemeFlag", true)]
        }
        ("CodeEdit", "add_code_completion_option") => {
            &[("location", "CodeEdit.CodeCompletionLocation", false)]
        }
        #[cfg(before_api = "4.6")] // https://github.com/godotengine/godot/pull/114053.
        ("FileAccess", "create_temp") => &[("mode_flags", "FileAccess.ModeFlags", true)],
        ("GPUParticles2D", "emit_particle") => &[("flags", "GPUParticles2D.EmitFlags", true)],
        ("GPUParticles3D", "emit_particle") => &[("flags", "GPUParticles3D.EmitFlags", true)],
        ("Node", "duplicate") => &[("flags", "Node.DuplicateFlags", true)],
        ("ProgressBar", "get_fill_mode") => &[("", "ProgressBar.FillMode", false)],
        ("ProgressBar", "set_fill_mode") => &[("mode", "ProgressBar.FillMode", false)],
        ("TextEdit", "search") => &[("flags", "TextEdit.SearchFlags", true)],
        ("TextEdit", "set_search_flags") => &[("flags", "TextEdit.SearchFlags", true)],
        ("TextureProgressBar", "get_fill_mode") => &[("", "TextureProgressBar.FillMode", false)],
        ("TextureProgressBar", "set_fill_mode") => {
            &[("mode", "TextureProgressBar.FillMode", false)]
        }
        ("Tree", "get_drop_mode_flags") => &[("", "Tree.DropModeFlags", true)],
        ("Tree", "set_drop_mode_flags") => &[("flags", "Tree.DropModeFlags", true)],

        // TODO(v0.5):
        // ("FBXDocument" | "GLTFDocument" | , "append_from_buffer" | "append_from_file" | "append_from_scene") => {
        //    Maps to a "quasy-bitfield": EditorSceneFormatImporter has constants such as IMPORT_USE_NAMED_SKIN_BEADS, but not an actual enum.
        //    See https://godot-rust.github.io/docs/gdext/master/godot/classes/struct.EditorSceneFormatImporter.html.
        //    Note that FBXDocument inherits GLTFDocument, but the methods are exposed twice.
        // }
        _ => &[],
    }
}

/// Returns whether a builtin method appears directly in the outer, public API (as opposed to private in `Inner*` structs).
///
/// For methods with default parameters, this also changes the signature to have an `*_ex` overload + `Ex*` builder struct. This is not done for
/// inner methods by default, to save on code generation.
#[rustfmt::skip]
pub fn is_builtin_method_exposed(builtin_ty: &TyName, godot_method_name: &str) -> bool {
    match (builtin_ty.godot_ty.as_str(), godot_method_name) {
        // GString/StringName shared methods.
        | ("String" | "StringName", "begins_with")
        | ("String" | "StringName", "bigrams")
        | ("String" | "StringName", "bin_to_int")
        | ("String" | "StringName", "c_escape")
        | ("String" | "StringName", "c_unescape")
        | ("String" | "StringName", "capitalize")
        | ("String" | "StringName", "contains")
        | ("String" | "StringName", "containsn")
        | ("String" | "StringName", "dedent")
        | ("String" | "StringName", "ends_with")
        | ("String" | "StringName", "get_base_dir")
        | ("String" | "StringName", "get_basename")
        | ("String" | "StringName", "get_extension")
        | ("String" | "StringName", "get_file")
        | ("String" | "StringName", "hex_decode")
        | ("String" | "StringName", "hex_to_int")
        | ("String" | "StringName", "indent")
        | ("String" | "StringName", "is_absolute_path")
        | ("String" | "StringName", "is_empty")
        | ("String" | "StringName", "is_relative_path")
        | ("String" | "StringName", "is_subsequence_of")
        | ("String" | "StringName", "is_subsequence_ofn")
        | ("String" | "StringName", "is_valid_filename")
        | ("String" | "StringName", "is_valid_float")
        | ("String" | "StringName", "is_valid_hex_number")
        | ("String" | "StringName", "is_valid_html_color")
        | ("String" | "StringName", "is_valid_identifier")
        | ("String" | "StringName", "is_valid_int")
        | ("String" | "StringName", "is_valid_ip_address")
        | ("String" | "StringName", "join")
        | ("String" | "StringName", "json_escape")
        | ("String" | "StringName", "left")
        | ("String" | "StringName", "lstrip")
        | ("String" | "StringName", "md5_buffer")
        | ("String" | "StringName", "md5_text")
        | ("String" | "StringName", "path_join")
        | ("String" | "StringName", "repeat")
        | ("String" | "StringName", "replace")
        | ("String" | "StringName", "replacen")
        | ("String" | "StringName", "reverse")
        | ("String" | "StringName", "right")
        | ("String" | "StringName", "rstrip")
        | ("String" | "StringName", "sha1_buffer")
        | ("String" | "StringName", "sha1_text")
        | ("String" | "StringName", "sha256_buffer")
        | ("String" | "StringName", "sha256_text")
        | ("String" | "StringName", "similarity")
        | ("String" | "StringName", "simplify_path")
        | ("String" | "StringName", "split_floats")
        | ("String" | "StringName", "strip_edges")
        | ("String" | "StringName", "strip_escapes")
        | ("String" | "StringName", "to_ascii_buffer")
        | ("String" | "StringName", "to_camel_case")
        | ("String" | "StringName", "to_float")
        | ("String" | "StringName", "to_int")
        | ("String" | "StringName", "to_lower")
        | ("String" | "StringName", "to_pascal_case")
        | ("String" | "StringName", "to_snake_case")
        | ("String" | "StringName", "to_upper")
        | ("String" | "StringName", "to_utf16_buffer")
        | ("String" | "StringName", "to_utf32_buffer")
        | ("String" | "StringName", "to_utf8_buffer")
        | ("String" | "StringName", "to_wchar_buffer")
        | ("String" | "StringName", "trim_prefix")
        | ("String" | "StringName", "trim_suffix")
        | ("String" | "StringName", "uri_decode")
        | ("String" | "StringName", "uri_encode")
        | ("String" | "StringName", "validate_filename")
        | ("String" | "StringName", "validate_node_name")
        | ("String" | "StringName", "xml_escape")
        | ("String" | "StringName", "xml_unescape")

        // GString
        | ("String", "chr")
        | ("String", "humanize_size")
        | ("String", "num")
        | ("String", "num_int64")
        | ("String", "num_scientific")
        | ("String", "num_uint64")

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

        // Basis
        | ("Basis", "looking_at")
        | ("Transform3D", "looking_at")

        => true, _ => false
    }
}

#[rustfmt::skip]
pub fn is_method_excluded_from_default_params(class_or_builtin_ty: Option<&TyName>, godot_method_name: &str) -> bool {
    // Utility functions: use "" string.
    let class_name = class_or_builtin_ty.map_or("", |ty| ty.godot_ty.as_str());

    // Private methods don't need to generate extra code for default extender machinery.
    if let Some(ty) = class_or_builtin_ty {
        if is_method_private(ty, godot_method_name) {
            return true;
        }
    }

    match (class_name, godot_method_name) {
        // Class exclusions.
        | ("Object", "notification")

        // Builtin exclusions.
        // Do not add methods here that aren't also part of is_builtin_method_exposed(). Methods on Inner* structs
        // do not have default-parameter code generation.

        | ("String" | "StringName", "find")
        | ("String" | "StringName", "findn")
        | ("String" | "StringName", "rfind")
        | ("String" | "StringName", "rfindn")
        | ("String" | "StringName", "split")
        | ("String" | "StringName", "rsplit")

        | ("Array", "duplicate")
        | ("Array", "duplicate_deep")
        | ("Array", "slice")
        | ("Array", "find")
        | ("Array", "rfind")
        | ("Array", "find_custom")
        | ("Array", "rfind_custom")
        | ("Array", "bsearch")
        | ("Array", "bsearch_custom")
        | ("Array", "reduce")

        | ("Dictionary", "duplicate")

        // PackedByteArray-specific methods with custom wrappers.
        | ("PackedByteArray", "encode_var")
        | ("PackedByteArray", "decode_var")
        | ("PackedByteArray", "decode_var_size")
        | ("PackedByteArray", "compress")
        | ("PackedByteArray", "decompress")
        | ("PackedByteArray", "decompress_dynamic")

        => true,

        // Packed*Array common methods with custom wrappers (slice, find, rfind, bsearch)
        (builtin, "slice" | "find" | "rfind" | "bsearch")
            if builtin.starts_with("Packed") && builtin.ends_with("Array")
        => true,

        _ => false
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
        | ("MultiplayerApi", "has_multiplayer_peer")
        | ("MultiplayerApi", "get_multiplayer_peer")
        | ("MultiplayerApi", "get_unique_id")
        | ("MultiplayerApi", "is_server")
        | ("MultiplayerApi", "get_remote_sender_id")
        | ("MultiplayerApi", "get_peers")
        | ("MultiplayerApi", "get_default_interface")
        => Some(true),

        // Changed to mut.
        | ("EditorImportPlugin", "_import")
        => Some(false),

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
/// instead of `Option<Gd<T>>`. By default, parameters are optional since we don't have nullability information in GDExtension.
pub fn is_class_method_param_required(
    class_name: &TyName,
    godot_method_name: &str,
    param: &Ident, // Don't use `&str` to avoid to_string() allocations for each check on call-site.
) -> bool {
    // TODO(v0.5): this overlaps now slightly with Godot's own "required" meta in extension_api.json.
    // Having an override list can always be useful, but possibly the two inputs (here + JSON) should be evaluated at the same time,
    // during JSON->Domain mapping.

    // Note: magically, it's enough if a base class method is declared here; it will be picked up by derived classes.

    match (class_name.godot_ty.as_str(), godot_method_name) {
        // Nodes.
        ("Node", "_input") => true,
        ("Node", "_shortcut_input") => true,
        ("Node", "_unhandled_input") => true,
        ("Node", "_unhandled_key_input") => true,

        // https://docs.godotengine.org/en/stable/classes/class_collisionobject2d.html#class-collisionobject2d-private-method-input-event
        ("CollisionObject2D", "_input_event") => true, // both parameters.

        // UI.
        ("Control", "_gui_input") => true,

        // Script instances.
        ("ScriptExtension", "_instance_create") => param == "for_object",
        ("ScriptExtension", "_placeholder_instance_create") => param == "for_object",
        ("ScriptExtension", "_inherits_script") => param == "script",
        ("ScriptExtension", "_instance_has") => param == "object",

        // Editor.
        ("EditorExportPlugin", "_customize_resource") => param == "resource",
        ("EditorExportPlugin", "_customize_scene") => param == "scene",

        ("EditorPlugin", "_handles") => param == "object",

        _ => false,
    }
}

/// True if builtin method is excluded. Does NOT check for type exclusion; use [`is_builtin_type_deleted`] for that.
pub fn is_builtin_method_deleted(_class_name: &TyName, method: &JsonBuiltinMethod) -> bool {
    codegen_special_cases::is_builtin_method_excluded(method)
}

/// Returns some generic type – such as `GenericArray` representing `Array<T>` – if method is marked as generic, `None` otherwise.
///
/// Usually required to initialize the return value and cache its type (see also https://github.com/godot-rust/gdext/pull/1357).
#[rustfmt::skip]
pub fn builtin_method_generic_ret(
    class_name: &TyName,
    method: &JsonBuiltinMethod,
) -> Option<FnReturn> {
    match (
        class_name.rust_ty.to_string().as_str(),
        method.name.as_str(),
    ) {
        | ("Array", "duplicate")
        | ("Array", "slice")
        | ("Array", "filter")

        => Some(FnReturn::with_generic_builtin(RustTy::GenericArray)),
        _ => None,
    }
}

/// True if signal is absent from codegen (only when surrounding class is excluded).
pub fn is_signal_deleted(_class_name: &TyName, signal: &JsonSignal) -> bool {
    // If any argument type (a class) is excluded.
    option_as_slice(&signal.arguments)
        .iter()
        .any(|arg| codegen_special_cases::is_class_excluded(&arg.type_))
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
    let hardcoded = match function.name.as_str() {
        // Removed in v0.3, but available as dedicated APIs.
        | "instance_from_id"

        => true, _ => false
    };

    hardcoded || codegen_special_cases::is_utility_function_excluded(function, ctx)
}

#[rustfmt::skip]
pub fn is_utility_function_private(function: &JsonUtilityFunction) -> bool {
    match function.name.as_str() {
        // Removed from public interface in v0.3, but available as dedicated APIs.
        | "is_instance_valid"    // used in Variant::is_object_alive().
        | "is_instance_id_valid" // used in InstanceId::lookup_validity().

        => true, _ => false
    }
}

pub fn maybe_rename_class_method<'m>(
    class_name: &TyName,
    godot_method_name: &'m str,
) -> Cow<'m, str> {
    // This is for non-virtual methods only. For virtual methods, use other handler below.

    if is_class_method_replaced_with_type_safe(class_name, godot_method_name) {
        let new_name = format!("raw_{godot_method_name}");
        return Cow::Owned(new_name);
    }

    let hardcoded = match (class_name.godot_ty.as_str(), godot_method_name) {
        // GDScript class, possibly more in the future.
        (_, "new") => "instantiate",

        _ => godot_method_name,
    };

    Cow::Borrowed(hardcoded)
}

// Maybe merge with above?
pub fn maybe_rename_virtual_method<'m>(
    class_name: &TyName,
    godot_method_name: &'m str,
) -> Option<&'m str> {
    let rust_name = match (class_name.godot_ty.as_str(), godot_method_name) {
        // Should no longer be relevant (worked around https://github.com/godotengine/godot/pull/99181#issuecomment-2543311415).
        // ("AnimationNodeExtension", "_process") => "process_animation",

        // A few classes define a virtual method called "_init" (distinct from the constructor)
        // -> rename those to avoid a name conflict in I* interface trait.
        (_, "_init") => "init_ext",

        _ => return None,
    };

    Some(rust_name)
}

// TODO method-level extra docs, for:
// - Node::rpc_config() -> link to RpcConfig.
// - Node::process/physics_process -> mention `f32`/`f64` duality.
// - Node::duplicate -> to copy #[var] fields, needs STORAGE property usage, or #[export],
//   or #[export(storage)] which is #[export] without editor UI.

pub fn get_class_extra_docs(class_name: &TyName) -> Option<&'static str> {
    match class_name.godot_ty.as_str() {
        "FileAccess" => {
            Some("The gdext library provides a higher-level abstraction, which should be preferred: [`GFile`][crate::tools::GFile].")
        }
        "ScriptExtension" => {
            Some("Use this in combination with the [`obj::script` module][crate::obj::script].")
        }
        "ResourceFormatLoader" => {
            Some("Enable the `experimental-threads` feature when using custom `ResourceFormatLoader`s. \
            Otherwise the application will panic when the custom `ResourceFormatLoader` is used by Godot \
            in a thread other than the main thread.")
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

#[cfg(before_api = "4.4")]
pub fn is_virtual_method_required(class_name: &TyName, godot_method_name: &str) -> bool {
    // Do not call is_derived_virtual_method_required() here; that is handled in virtual_traits.rs.

    match (class_name.godot_ty.as_str(), godot_method_name) {
        ("ScriptLanguageExtension", method) => method != "_get_doc_comment_delimiters",

        ("ScriptExtension", "_editor_can_reload_from_file")
        | ("ScriptExtension", "_can_instantiate")
        | ("ScriptExtension", "_get_base_script")
        | ("ScriptExtension", "_get_global_name")
        | ("ScriptExtension", "_inherits_script")
        | ("ScriptExtension", "_get_instance_base_type")
        | ("ScriptExtension", "_instance_create")
        | ("ScriptExtension", "_placeholder_instance_create")
        | ("ScriptExtension", "_instance_has")
        | ("ScriptExtension", "_has_source_code")
        | ("ScriptExtension", "_get_source_code")
        | ("ScriptExtension", "_set_source_code")
        | ("ScriptExtension", "_reload")
        | ("ScriptExtension", "_get_documentation")
        | ("ScriptExtension", "_has_method")
        | ("ScriptExtension", "_has_static_method")
        | ("ScriptExtension", "_get_method_info")
        | ("ScriptExtension", "_is_tool")
        | ("ScriptExtension", "_is_valid")
        | ("ScriptExtension", "_get_language")
        | ("ScriptExtension", "_has_script_signal")
        | ("ScriptExtension", "_get_script_signal_list")
        | ("ScriptExtension", "_has_property_default_value")
        | ("ScriptExtension", "_get_property_default_value")
        | ("ScriptExtension", "_update_exports")
        | ("ScriptExtension", "_get_script_method_list")
        | ("ScriptExtension", "_get_script_property_list")
        | ("ScriptExtension", "_get_member_line")
        | ("ScriptExtension", "_get_constants")
        | ("ScriptExtension", "_get_members")
        | ("ScriptExtension", "_is_placeholder_fallback_enabled")
        | ("ScriptExtension", "_get_rpc_config")
        | ("EditorExportPlugin", "_customize_resource")
        | ("EditorExportPlugin", "_customize_scene")
        | ("EditorExportPlugin", "_get_customization_configuration_hash")
        | ("EditorExportPlugin", "_get_name")
        | ("EditorVCSInterface", _)
        | ("MovieWriter", _)
        | ("TextServerExtension", "_has_feature")
        | ("TextServerExtension", "_get_name")
        | ("TextServerExtension", "_get_features")
        | ("TextServerExtension", "_free_rid")
        | ("TextServerExtension", "_has")
        | ("TextServerExtension", "_create_font")
        | ("TextServerExtension", "_font_set_fixed_size")
        | ("TextServerExtension", "_font_get_fixed_size")
        | ("TextServerExtension", "_font_set_fixed_size_scale_mode")
        | ("TextServerExtension", "_font_get_fixed_size_scale_mode")
        | ("TextServerExtension", "_font_get_size_cache_list")
        | ("TextServerExtension", "_font_clear_size_cache")
        | ("TextServerExtension", "_font_remove_size_cache")
        | ("TextServerExtension", "_font_set_ascent")
        | ("TextServerExtension", "_font_get_ascent")
        | ("TextServerExtension", "_font_set_descent")
        | ("TextServerExtension", "_font_get_descent")
        | ("TextServerExtension", "_font_set_underline_position")
        | ("TextServerExtension", "_font_get_underline_position")
        | ("TextServerExtension", "_font_set_underline_thickness")
        | ("TextServerExtension", "_font_get_underline_thickness")
        | ("TextServerExtension", "_font_set_scale")
        | ("TextServerExtension", "_font_get_scale")
        | ("TextServerExtension", "_font_get_texture_count")
        | ("TextServerExtension", "_font_clear_textures")
        | ("TextServerExtension", "_font_remove_texture")
        | ("TextServerExtension", "_font_set_texture_image")
        | ("TextServerExtension", "_font_get_texture_image")
        | ("TextServerExtension", "_font_get_glyph_list")
        | ("TextServerExtension", "_font_clear_glyphs")
        | ("TextServerExtension", "_font_remove_glyph")
        | ("TextServerExtension", "_font_get_glyph_advance")
        | ("TextServerExtension", "_font_set_glyph_advance")
        | ("TextServerExtension", "_font_get_glyph_offset")
        | ("TextServerExtension", "_font_set_glyph_offset")
        | ("TextServerExtension", "_font_get_glyph_size")
        | ("TextServerExtension", "_font_set_glyph_size")
        | ("TextServerExtension", "_font_get_glyph_uv_rect")
        | ("TextServerExtension", "_font_set_glyph_uv_rect")
        | ("TextServerExtension", "_font_get_glyph_texture_idx")
        | ("TextServerExtension", "_font_set_glyph_texture_idx")
        | ("TextServerExtension", "_font_get_glyph_texture_rid")
        | ("TextServerExtension", "_font_get_glyph_texture_size")
        | ("TextServerExtension", "_font_get_glyph_index")
        | ("TextServerExtension", "_font_get_char_from_glyph_index")
        | ("TextServerExtension", "_font_has_char")
        | ("TextServerExtension", "_font_get_supported_chars")
        | ("TextServerExtension", "_font_draw_glyph")
        | ("TextServerExtension", "_font_draw_glyph_outline")
        | ("TextServerExtension", "_create_shaped_text")
        | ("TextServerExtension", "_shaped_text_clear")
        | ("TextServerExtension", "_shaped_text_add_string")
        | ("TextServerExtension", "_shaped_text_add_object")
        | ("TextServerExtension", "_shaped_text_resize_object")
        | ("TextServerExtension", "_shaped_get_span_count")
        | ("TextServerExtension", "_shaped_get_span_meta")
        | ("TextServerExtension", "_shaped_set_span_update_font")
        | ("TextServerExtension", "_shaped_text_substr")
        | ("TextServerExtension", "_shaped_text_get_parent")
        | ("TextServerExtension", "_shaped_text_shape")
        | ("TextServerExtension", "_shaped_text_is_ready")
        | ("TextServerExtension", "_shaped_text_get_glyphs")
        | ("TextServerExtension", "_shaped_text_sort_logical")
        | ("TextServerExtension", "_shaped_text_get_glyph_count")
        | ("TextServerExtension", "_shaped_text_get_range")
        | ("TextServerExtension", "_shaped_text_get_trim_pos")
        | ("TextServerExtension", "_shaped_text_get_ellipsis_pos")
        | ("TextServerExtension", "_shaped_text_get_ellipsis_glyphs")
        | ("TextServerExtension", "_shaped_text_get_ellipsis_glyph_count")
        | ("TextServerExtension", "_shaped_text_get_objects")
        | ("TextServerExtension", "_shaped_text_get_object_rect")
        | ("TextServerExtension", "_shaped_text_get_object_range")
        | ("TextServerExtension", "_shaped_text_get_object_glyph")
        | ("TextServerExtension", "_shaped_text_get_size")
        | ("TextServerExtension", "_shaped_text_get_ascent")
        | ("TextServerExtension", "_shaped_text_get_descent")
        | ("TextServerExtension", "_shaped_text_get_width")
        | ("TextServerExtension", "_shaped_text_get_underline_position")
        | ("TextServerExtension", "_shaped_text_get_underline_thickness")
        | ("AudioStreamPlayback", "_mix")
        | ("AudioStreamPlaybackResampled", "_mix_resampled")
        | ("AudioStreamPlaybackResampled", "_get_stream_sampling_rate")
        | ("AudioEffectInstance", "_process")
        | ("AudioEffect", "_instantiate")
        | ("PhysicsDirectBodyState2DExtension", _)
        | ("PhysicsDirectBodyState3DExtension", _)
        | ("PhysicsDirectSpaceState2DExtension", _)
        | ("PhysicsDirectSpaceState3DExtension", _)
        | ("PhysicsServer3DExtension", _)
        | ("PhysicsServer2DExtension", _)
        | ("EditorScript", "_run")
        | ("VideoStreamPlayback", "_update")
        | ("EditorFileSystemImportFormatSupportQuery", _)
        | ("Mesh", _)
        | ("Texture2D", "_get_width")
        | ("Texture2D", "_get_height")
        | ("Texture3D", _)
        | ("TextureLayered", _)
        | ("StyleBox", "_draw")
        | ("Material", "_get_shader_rid")
        | ("Material", "_get_shader_mode")
        | ("PacketPeerExtension", "_get_available_packet_count")
        | ("PacketPeerExtension", "_get_max_packet_size")
        | ("StreamPeerExtension", "_get_available_bytes")
        | ("WebRTCDataChannelExtension", "_poll")
        | ("WebRTCDataChannelExtension", "_close")
        | ("WebRTCDataChannelExtension", "_set_write_mode")
        | ("WebRTCDataChannelExtension", "_get_write_mode")
        | ("WebRTCDataChannelExtension", "_was_string_packet")
        | ("WebRTCDataChannelExtension", "_get_ready_state")
        | ("WebRTCDataChannelExtension", "_get_label")
        | ("WebRTCDataChannelExtension", "_is_ordered")
        | ("WebRTCDataChannelExtension", "_get_id")
        | ("WebRTCDataChannelExtension", "_get_max_packet_life_time")
        | ("WebRTCDataChannelExtension", "_get_max_retransmits")
        | ("WebRTCDataChannelExtension", "_get_protocol")
        | ("WebRTCDataChannelExtension", "_is_negotiated")
        | ("WebRTCDataChannelExtension", "_get_buffered_amount")
        | ("WebRTCDataChannelExtension", "_get_available_packet_count")
        | ("WebRTCDataChannelExtension", "_get_max_packet_size")
        | ("WebRTCPeerConnectionExtension", _)
        | ("MultiplayerPeerExtension", "_get_available_packet_count")
        | ("MultiplayerPeerExtension", "_get_max_packet_size")
        | ("MultiplayerPeerExtension", "_set_transfer_channel")
        | ("MultiplayerPeerExtension", "_get_transfer_channel")
        | ("MultiplayerPeerExtension", "_set_transfer_mode")
        | ("MultiplayerPeerExtension", "_get_transfer_mode")
        | ("MultiplayerPeerExtension", "_set_target_peer")
        | ("MultiplayerPeerExtension", "_get_packet_peer")
        | ("MultiplayerPeerExtension", "_get_packet_mode")
        | ("MultiplayerPeerExtension", "_get_packet_channel")
        | ("MultiplayerPeerExtension", "_is_server")
        | ("MultiplayerPeerExtension", "_poll")
        | ("MultiplayerPeerExtension", "_close")
        | ("MultiplayerPeerExtension", "_disconnect_peer")
        | ("MultiplayerPeerExtension", "_get_unique_id")
        | ("MultiplayerPeerExtension", "_get_connection_status") => true,

        _ => false,
    }
}

// Adjustments for Godot 4.4+, where a virtual method is no longer needed (e.g. in a derived class).
#[rustfmt::skip]
pub fn get_derived_virtual_method_presence(class_name: &TyName, godot_method_name: &str) -> VirtualMethodPresence {
     match (class_name.godot_ty.as_str(), godot_method_name) {
         // Required in base class, no longer in derived; https://github.com/godot-rust/gdext/issues/1133.
         | ("AudioStreamPlaybackResampled", "_mix")
         => VirtualMethodPresence::Remove,

         | ("PrimitiveMesh", "_get_surface_count")
         | ("PrimitiveMesh", "_surface_get_array_len")
         | ("PrimitiveMesh", "_surface_get_array_index_len")
         | ("PrimitiveMesh", "_surface_get_arrays")
         | ("PrimitiveMesh", "_surface_get_blend_shape_arrays")
         | ("PrimitiveMesh", "_surface_get_lods")
         | ("PrimitiveMesh", "_surface_get_format")
         | ("PrimitiveMesh", "_surface_get_primitive_type")
         | ("PrimitiveMesh", "_surface_set_material")
         | ("PrimitiveMesh", "_surface_get_material")
         | ("PrimitiveMesh", "_get_blend_shape_count")
         | ("PrimitiveMesh", "_get_blend_shape_name")
         | ("PrimitiveMesh", "_set_blend_shape_name")
         | ("PrimitiveMesh", "_get_aabb")
         => VirtualMethodPresence::Override { is_required: false },

         // Methods which are required but not marked as such.
         // https://docs.godotengine.org/en/stable/classes/class_editorsyntaxhighlighter.html#class-editorsyntaxhighlighter-private-method-create
         | ("EditorSyntaxHighlighter", "_create") // https://github.com/godot-rust/gdext/issues/1452.
         => VirtualMethodPresence::Override { is_required: true },

         // Default: inherit presence from base class.
         _ => VirtualMethodPresence::Inherit,
    }
}

/// Initialization order for Godot (see https://github.com/godotengine/godot/blob/master/main/main.cpp).
/// - Main::setup()
///   - register_core_types()
///   - register_early_core_singletons()
///   - initialize_extensions(GDExtension::INITIALIZATION_LEVEL_CORE)
/// - Main::setup2()
///   - register_server_types()
///   - initialize_extensions(GDExtension::INITIALIZATION_LEVEL_SERVERS)
///   - register_core_singletons() ...possibly a bug. Should this be before LEVEL_SERVERS?
///   - register_scene_types()
///   - register_scene_singletons()
///   - initialize_extensions(GDExtension::INITIALIZATION_LEVEL_SCENE)
///   - IF EDITOR
///     - register_editor_types()
///     - initialize_extensions(GDExtension::INITIALIZATION_LEVEL_EDITOR)
///   - register_server_singletons() ...another weird one.
///   - Autoloads, etc.
///
/// ## Singleton availability by initialization level
/// - **Core level**: Basic singletons like `Engine`, `OS`, `ProjectSettings`, `Time` are available.
/// - **Servers level**: Server singletons like `RenderingServer` are NOT yet available due to GDExtension timing issues.
/// - **Scene level**: All singletons including `RenderingServer` are available.
/// - **Editor level**: Editor-specific functionality is available.
///
/// GDExtension singletons are generally not available during *any* level initialization, with the exception of a few core singletons 
/// (see above). This is different from how modules work, where servers are available at _Servers_ level.
///
/// See also:
/// - Singletons not accessible in Scene (godot-cpp): <https://github.com/godotengine/godot-cpp/issues/1180>
/// - `global_get_singleton` not returning singletons: <https://github.com/godotengine/godot/issues/64975>
/// - PR to make singletons available: <https://github.com/godotengine/godot/pull/98862>
#[rustfmt::skip]
pub fn classify_codegen_level(class_name: &str) -> Option<ClassCodegenLevel> {
    let level = match class_name {
        // See register_core_types() in https://github.com/godotengine/godot/blob/master/core/register_core_types.cpp,
        // which is called before Core level is initialized. Only a small list is promoted to Core; carefully evaluate if more are added.
        | "Object" | "RefCounted" | "Resource" | "MainLoop" | "GDExtension"
        => ClassCodegenLevel::Core,

        // See register_early_core_singletons() in https://github.com/godotengine/godot/blob/master/core/register_core_types.cpp,
        // which is called before Core level is initialized.
        // ClassDB is available, however its *singleton* will be registered at Core level only from Godot 4.7 on, see
        // https://github.com/godot-rust/gdext/pull/1474. Its function pointers can already be fetched in Core before; there's just no instance.
        | "ProjectSettings" | "Engine" | "OS" | "Time" | "ClassDB"
        => ClassCodegenLevel::Core,

        // See initialize_openxr_module() in https://github.com/godotengine/godot/blob/master/modules/openxr/register_types.cpp
        | "OpenXRExtensionWrapper"
        => ClassCodegenLevel::Core,

        // Symbols from another extension could be available in Core, but since GDExtension can currently not guarantee
        // the order of different extensions being loaded, we prevent implicit dependencies and require Server.
        | "OpenXRExtensionWrapperExtension"
        => ClassCodegenLevel::Servers,

        // See register_server_types() in https://github.com/godotengine/godot/blob/master/servers/register_server_types.cpp
        | "PhysicsDirectBodyState2D" | "PhysicsDirectBodyState2DExtension" 
        | "PhysicsDirectSpaceState2D" | "PhysicsDirectSpaceState2DExtension" 
        | "PhysicsServer2D" | "PhysicsServer2DExtension" 
        | "PhysicsServer2DManager" 
        | "PhysicsDirectBodyState3D" | "PhysicsDirectBodyState3DExtension" 
        | "PhysicsDirectSpaceState3D" | "PhysicsDirectSpaceState3DExtension" 
        | "PhysicsServer3D" | "PhysicsServer3DExtension" 
        | "PhysicsServer3DManager" 
        | "PhysicsServer3DRenderingServerHandler"
        | "RenderData" | "RenderDataExtension"
        | "RenderSceneData" | "RenderSceneDataExtension"
        => ClassCodegenLevel::Servers,
        
        // Declared final (un-inheritable) in Rust, but those are still servers.
        | "AudioServer" | "CameraServer" | "NavigationServer2D" | "NavigationServer3D" | "RenderingServer" | "TranslationServer" | "XRServer" | "DisplayServer"
        => ClassCodegenLevel::Servers,

        // Work around wrong classification in https://github.com/godotengine/godot/issues/86206.
        // https://github.com/godotengine/godot/issues/103867
        "OpenXRInteractionProfileEditorBase"
        | "OpenXRInteractionProfileEditor"
        | "OpenXRBindingModifierEditor" if cfg!(before_api = "4.5") 
        => ClassCodegenLevel::Editor,
        
        // https://github.com/godotengine/godot/issues/86206
        "ResourceImporterOggVorbis" | "ResourceImporterMP3" if cfg!(before_api = "4.3") 
        => ClassCodegenLevel::Editor,

        // No special-case override for this class.
        _ => return None,
    };
    Some(level)
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

/// Overrides Godot's enum/bitfield status.
/// * `Some(true)` -> bitfield
/// * `Some(false)` -> enum
/// * `None` -> keep default
#[rustfmt::skip]
pub fn is_enum_bitfield(class_name: Option<&TyName>, enum_name: &str) -> Option<bool> {
    let class_name = class_name.map(|c| c.godot_ty.as_str());
    match (class_name, enum_name) {
        | (Some("FileAccess"), "ModeFlags")
        | (Some("GPUParticles2D"), "EmitFlags")
        | (Some("GPUParticles3D"), "EmitFlags")
        | (Some("Node"), "DuplicateFlags")
        | (Some("Object"), "ConnectFlags")
        | (Some("SceneTree"), "GroupCallFlags")
        | (Some("TextEdit"), "SearchFlags")

        => Some(true),
        _ => None
    }
}

/// Whether an enum can be combined with another enum (return value) for bitmasking purposes.
///
/// If multiple masks are ever necessary, this can be extended to return a slice instead of Option.
///
/// If a mapping is found, returns the corresponding `RustTy`.
pub fn as_enum_bitmaskable(enum_: &Enum) -> Option<RustTy> {
    if enum_.is_bitfield {
        // Only enums need this treatment.
        return None;
    }

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
