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

// NOTE: the identifiers used here operate on the GODOT types (e.g. AABB, not Aabb)

#[rustfmt::skip]
pub fn is_deleted(godot_class_name: &str, godot_method_name: &str) -> bool {
    match (godot_class_name, godot_method_name) {
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
pub fn is_class_deleted(godot_class_name: &str) -> bool {
    match godot_class_name {
        // Thread APIs
        | "Thread"
        | "Mutex"
        | "Semaphore"

        => true, _ => false
    }
}

#[rustfmt::skip]
pub fn is_private(godot_class_name: &str, godot_method_name: &str) -> bool {
    match (godot_class_name, godot_method_name) {
        // Already covered by manual APIs
        | ("Object", "to_string")
        | ("RefCounted", "init_ref")
        | ("RefCounted", "reference")
        | ("RefCounted", "unreference")

        => true, _ => false
    }
}

pub fn is_builtin_type_deleted(godot_class_name: &str) -> bool {
    godot_class_name == "Nil"
        || godot_class_name
            .chars()
            .next()
            .unwrap()
            .is_ascii_lowercase()
}

pub fn maybe_renamed<'m>(godot_class_name: &str, godot_method_name: &'m str) -> &'m str {
    match (godot_class_name, godot_method_name) {
        // GDScript, GDScriptNativeClass, possibly more in the future
        (_, "new") => "instantiate",
        _ => godot_method_name,
    }
}
