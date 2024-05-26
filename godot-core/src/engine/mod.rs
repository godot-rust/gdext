/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Godot engine classes and methods.

use crate::builtin::GString;
use crate::obj::{bounds, Bounds, Gd, GodotClass, InstanceId};

// Re-exports of generated symbols

pub use crate::gen::classes::*;

use crate::builtin::meta::CallContext;
use crate::sys;

mod manual_extensions;

#[cfg(debug_assertions)]
use crate::builtin::meta::ClassName;

/// Support for Godot _native structures_.
///
/// Native structures are a niche API in Godot. These are low-level data types that are passed as pointers to/from the engine.
/// In Rust, they are represented as `#[repr(C)]` structs.
///
/// There is unfortunately not much official documentation available; you may need to look at Godot source code.
/// Most users will not need native structures, as they are very specialized.
pub mod native {
    pub use crate::gen::native::*;
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Deprecations

#[deprecated = "Enums have been moved to `godot::global`."]
pub mod global {
    pub use crate::builtin::{Corner, EulerOrder, Side};
    pub use crate::global::*;
}

#[deprecated = "Utility functions have been moved to `godot::global`."]
pub mod utilities {
    pub use crate::global::*;
}

#[deprecated = "`godot::engine::translate` has been moved to `godot::extras`."]
pub mod translate {
    pub use crate::extras::{tr, tr_n};
}

#[deprecated = "`create_script_instance` has been moved to `godot::extras`."]
pub use crate::extras::create_script_instance;

#[deprecated = "`ScriptInstance` has been moved to `godot::extras`."]
pub use crate::extras::ScriptInstance;

#[deprecated = "`SiMut` has been moved to `godot::extras`."]
pub use crate::extras::SiMut;

#[deprecated = "`GFile` has been moved to `godot::extras`."]
pub use crate::extras::GFile;

#[deprecated = "`IoError` has been moved to `godot::extras`."]
pub use crate::extras::IoError;

#[deprecated = "`save` has been moved to `godot::global`."]
pub use crate::global::save;

#[deprecated = "`try_save` has been moved to `godot::global`."]
pub use crate::global::try_save;

#[deprecated = "`load` has been moved to `godot::global`."]
pub use crate::global::load;

#[deprecated = "`try_load` has been moved to `godot::global`."]
pub use crate::global::try_load;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Utilities for crate

pub(crate) fn debug_string<T: GodotClass>(
    obj: &Gd<T>,
    f: &mut std::fmt::Formatter<'_>,
    ty: &str,
) -> std::fmt::Result {
    if let Some(id) = obj.instance_id_or_none() {
        let class: GString = obj.raw.as_object().get_class();
        write!(f, "{ty} {{ id: {id}, class: {class} }}")
    } else {
        write!(f, "{ty} {{ freed obj }}")
    }
}

pub(crate) fn display_string<T: GodotClass>(
    obj: &Gd<T>,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    let string: GString = obj.raw.as_object().to_string();
    <GString as std::fmt::Display>::fmt(&string, f)
}

pub(crate) fn object_ptr_from_id(instance_id: InstanceId) -> sys::GDExtensionObjectPtr {
    // SAFETY: Godot looks up ID in ObjectDB and returns null if not found.
    unsafe { sys::interface_fn!(object_get_instance_from_id)(instance_id.to_u64()) }
}

pub(crate) fn construct_engine_object<T>() -> Gd<T>
where
    T: GodotClass + Bounds<Declarer = bounds::DeclEngine>,
{
    // SAFETY: adhere to Godot API; valid class name and returned pointer is an object.
    unsafe {
        let object_ptr = sys::interface_fn!(classdb_construct_object)(T::class_name().string_sys());
        Gd::from_obj_sys(object_ptr)
    }
}

pub(crate) fn ensure_object_alive(
    instance_id: InstanceId,
    old_object_ptr: sys::GDExtensionObjectPtr,
    call_ctx: &CallContext,
) {
    let new_object_ptr = object_ptr_from_id(instance_id);

    assert!(
        !new_object_ptr.is_null(),
        "{call_ctx}: access to instance with ID {instance_id} after it has been freed"
    );

    // This should not happen, as reuse of instance IDs was fixed according to https://github.com/godotengine/godot/issues/32383,
    // namely in PR https://github.com/godotengine/godot/pull/36189. Double-check to make sure.
    assert_eq!(
        new_object_ptr, old_object_ptr,
        "{call_ctx}: instance ID {instance_id} points to a stale, reused object. Please report this to gdext maintainers."
    );
}

#[cfg(debug_assertions)]
pub(crate) fn ensure_object_inherits(
    derived: ClassName,
    base: ClassName,
    instance_id: InstanceId,
) -> bool {
    if derived == base
        || base == Object::class_name() // for Object base, anything inherits by definition
        || is_derived_base_cached(derived, base)
    {
        return true;
    }

    panic!(
        "Instance of ID {instance_id} has type {derived} but is incorrectly stored in a Gd<{base}>.\n\
        This may happen if you change an object's identity through DerefMut."
    )
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Implementation of this file

/// Checks if `derived` inherits from `base`, using a cache for _successful_ queries.
#[cfg(debug_assertions)]
fn is_derived_base_cached(derived: ClassName, base: ClassName) -> bool {
    use std::collections::HashSet;
    use sys::Global;
    static CACHE: Global<HashSet<(ClassName, ClassName)>> = Global::default();

    let mut cache = CACHE.lock();
    let key = (derived, base);
    if cache.contains(&key) {
        return true;
    }

    // Query Godot API (takes linear time in depth of inheritance tree).
    let is_parent_class =
        ClassDb::singleton().is_parent_class(derived.to_string_name(), base.to_string_name());

    // Insert only successful queries. Those that fail are on the error path already and don't need to be fast.
    if is_parent_class {
        cache.insert(key);
    }

    is_parent_class
}
