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

#[rustfmt::skip]
pub fn is_deleted(class_name: &str, method_name: &str) -> bool {
    match (class_name, method_name) {
        // Already covered by manual APIs
        //| ("Object", "to_string")
        | ("Object", "get_instance_id")

        => true, _ => false
    }
}

#[rustfmt::skip]
pub fn is_class_deleted(class_name: &str) -> bool {
    match class_name {
        | "Thread"
        | "Mutex"
        | "Semaphore"

        => true, _ => false
    }
}

#[rustfmt::skip]
pub fn is_private(class_name: &str, method_name: &str) -> bool {
    match (class_name, method_name) {
        // Already covered by manual APIs
        | ("Object", "to_string")
        | ("RefCounted", "init_ref")
        | ("RefCounted", "reference")
        | ("RefCounted", "unreference")

        => true, _ => false
    }
}
