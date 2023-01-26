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

use crate::TyName;

#[rustfmt::skip]
pub(crate) fn is_deleted(class_name: &TyName, godot_method_name: &str) -> bool {
    match (class_name.godot_ty.as_str(), godot_method_name) {
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
    match class_name.godot_ty.as_str() {
        // Thread APIs
        | "Thread"
        | "Mutex"
        | "Semaphore"

        => true, _ => false
    }
}

#[rustfmt::skip]
pub(crate) fn is_private(class_name: &TyName, godot_method_name: &str) -> bool {
    match (class_name.godot_ty.as_str(), godot_method_name) {
        // Already covered by manual APIs
        | ("Object", "to_string")
        | ("RefCounted", "init_ref")
        | ("RefCounted", "reference")
        | ("RefCounted", "unreference")

        => true, _ => false
    }
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
