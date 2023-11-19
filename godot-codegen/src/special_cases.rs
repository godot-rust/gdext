/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// Lists all cases in the Godot class API, where deviations are considered appropriate (e.g. for safety).

// Open design decisions:
// * Should Godot types like Node3D have all the "obj level" methods like to_string(), get_instance_id(), etc; or should those
//   be reserved for the Gd<T> pointer? The latter seems like a limitation. User objects also have to_string() (but not get_instance_id())
//   through the GodotExt trait. This could be unified.
// * The deleted/private methods and classes deemed "dangerous" may be provided later as unsafe functions -- our safety model
//   needs to first mature a bit.

// NOTE: the methods are generally implemented on Godot types (e.g. AABB, not Aabb)

#![allow(clippy::match_like_matches_macro)] // if there is only one rule

use crate::api_parser::{BuiltinClassMethod, ClassMethod};
use crate::Context;
use crate::{codegen_special_cases, TyName};

#[rustfmt::skip]
pub(crate) fn is_deleted(class_name: &TyName, method: &ClassMethod, ctx: &mut Context) -> bool {
    if codegen_special_cases::is_method_excluded(method, false, ctx){
        return true;
    }
    
    match (class_name.godot_ty.as_str(), method.name.as_str()) {
        // Already covered by manual APIs
        //| ("Object", "to_string")
        | ("Object", "get_instance_id")

        // Thread APIs
        | ("ResourceLoader", "load_threaded_get")
        | ("ResourceLoader", "load_threaded_get_status")
        | ("ResourceLoader", "load_threaded_request")
        // also: enum ThreadLoadStatus

        => true, _ => false
    }
}

#[rustfmt::skip]
pub(crate) fn is_class_deleted(class_name: &TyName) -> bool {
    // Exclude experimental APIs unless opted-in.
    if !cfg!(feature = "experimental-godot-api") && is_class_experimental(class_name) {
        return true;
    }

    let class_name = class_name.godot_ty.as_str();

    // OpenXR has not been available for macOS before 4.2.
    // See e.g. https://github.com/GodotVR/godot-xr-tools/issues/479.
    // Do not hardcode a list of OpenXR classes, as more may be added in future Godot versions; instead use prefix.
    #[cfg(all(before_api = "4.2", target_os = "macos"))]
    if class_name.starts_with("OpenXR") {
        return true;
    }

    // ThemeDB was previously loaded lazily
    // in 4.2 it loads at the Scene level
    // see: https://github.com/godotengine/godot/pull/81305
    #[cfg(before_api = "4.2")]
    if class_name == "ThemeDB" {
        return true;
    }

    match class_name {
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
        | "UniformSetCacheRD"

        => true, _ => false
    }
}

#[rustfmt::skip]
fn is_class_experimental(class_name: &TyName) -> bool {
    // These classes are currently hardcoded, but the information is available in Godot's doc/classes directory.
    // The XML file contains a property <class name="NavigationMesh" ... is_experimental="true">.

    match class_name.godot_ty.as_str() {
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
pub(crate) fn is_named_accessor_in_table(class_or_builtin_ty: &TyName, godot_method_name: &str) -> bool {
    // Generated methods made private are typically needed internally and exposed with a different API,
    // so make them accessible.
    is_private(class_or_builtin_ty, godot_method_name)
}

/// Whether a class or builtin method should be hidden from the public API.
#[rustfmt::skip]
pub(crate) fn is_private(class_or_builtin_ty: &TyName, godot_method_name: &str) -> bool {
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
pub(crate) fn is_excluded_from_default_params(class_name: Option<&TyName>, godot_method_name: &str) -> bool {
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
#[cfg(FALSE)] // TODO enable this once JSON/domain models are separated.
pub(crate) fn is_method_const(class_name: &TyName, godot_method: &ClassMethod) -> Option<bool> {
    match (class_name.godot_ty.as_str(), godot_method.name.as_str()) {
        // Changed to mut.
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

        _ => None,
    }
}

/// True if builtin method is excluded. Does NOT check for type exclusion; use [`is_builtin_type_deleted`] for that.
pub(crate) fn is_builtin_deleted(_class_name: &TyName, method: &BuiltinClassMethod) -> bool {
    // Currently only deleted if codegen.
    codegen_special_cases::is_builtin_method_excluded(method)
}

/// True if builtin type is excluded (`NIL` or scalars)
pub(crate) fn is_builtin_type_deleted(class_name: &TyName) -> bool {
    let name = class_name.godot_ty.as_str();
    name == "Nil" || is_builtin_scalar(name)
}

/// True if `int`, `float`, `bool`, ...
pub(crate) fn is_builtin_scalar(name: &str) -> bool {
    name.chars().next().unwrap().is_ascii_lowercase()
}

pub(crate) fn maybe_renamed<'m>(class_name: &TyName, godot_method_name: &'m str) -> &'m str {
    match (class_name.godot_ty.as_str(), godot_method_name) {
        // GDScript, GDScriptNativeClass, possibly more in the future
        (_, "new") => "instantiate",
        _ => godot_method_name,
    }
}
