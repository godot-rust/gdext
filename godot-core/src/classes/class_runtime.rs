/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Runtime checks and inspection of Godot classes.

use crate::builtin::{GString, StringName, Variant, VariantType};
#[cfg(debug_assertions)]
use crate::classes::{ClassDb, Object};
use crate::meta::CallContext;
#[cfg(debug_assertions)]
use crate::meta::ClassName;
use crate::obj::{bounds, Bounds, Gd, GodotClass, InstanceId, RawGd};
use crate::sys;

pub(crate) fn debug_string<T: GodotClass>(
    obj: &Gd<T>,
    f: &mut std::fmt::Formatter<'_>,
    ty: &str,
) -> std::fmt::Result {
    if let Some(id) = obj.instance_id_or_none() {
        let class: StringName = obj.dynamic_class_string();
        debug_string_parts(f, ty, id, class, obj.maybe_refcount(), None)
    } else {
        write!(f, "{ty} {{ freed obj }}")
    }
}

#[cfg(since_api = "4.4")]
pub(crate) fn debug_string_variant(
    obj: &Variant,
    f: &mut std::fmt::Formatter<'_>,
    ty: &str,
) -> std::fmt::Result {
    debug_assert_eq!(obj.get_type(), VariantType::OBJECT);

    let id = obj
        .object_id_unchecked()
        .expect("Variant must be of type OBJECT");

    if id.lookup_validity() {
        // Object::get_class() currently returns String, but this is future-proof if the return type changes to StringName.
        let class = obj
            .call("get_class", &[])
            .try_to_relaxed::<StringName>()
            .expect("get_class() must be compatible with StringName");

        let refcount = id.is_ref_counted().then(|| {
            obj.call("get_reference_count", &[])
                .try_to_relaxed::<i32>()
                .expect("get_reference_count() must return integer") as usize
        });

        debug_string_parts(f, ty, id, class, refcount, None)
    } else {
        write!(f, "{ty} {{ freed obj }}")
    }
}

// Polyfill for Godot < 4.4, where Variant::object_id_unchecked() is not available.
#[cfg(before_api = "4.4")]
pub(crate) fn debug_string_variant(
    obj: &Variant,
    f: &mut std::fmt::Formatter<'_>,
    ty: &str,
) -> std::fmt::Result {
    debug_assert_eq!(obj.get_type(), VariantType::OBJECT);

    match obj.try_to::<Gd<crate::classes::Object>>() {
        Ok(obj) => {
            let id = obj.instance_id(); // Guaranteed valid, since conversion would have failed otherwise.
            let class = obj.dynamic_class_string();

            // Refcount is off-by-one due to now-created Gd<T> from conversion; correct by -1.
            let refcount = obj.maybe_refcount().map(|rc| rc.saturating_sub(1));

            debug_string_parts(f, ty, id, class, refcount, None)
        }
        Err(_) => {
            write!(f, "{ty} {{ freed obj }}")
        }
    }
}

pub(crate) fn debug_string_nullable<T: GodotClass>(
    obj: &RawGd<T>,
    f: &mut std::fmt::Formatter<'_>,
    ty: &str,
) -> std::fmt::Result {
    if obj.is_null() {
        write!(f, "{ty} {{ null }}")
    } else {
        // Unsafety introduced here to avoid creating a new Gd<T> (which can have all sorts of side effects, logs, refcounts etc.)
        // *and* pushing down all high-level Gd<T> functions to RawGd<T> as pure delegates.

        // SAFETY: checked non-null.
        let obj = unsafe { obj.as_non_null() };
        debug_string(obj, f, ty)
    }
}

pub(crate) fn debug_string_with_trait<T: GodotClass>(
    obj: &Gd<T>,
    f: &mut std::fmt::Formatter<'_>,
    ty: &str,
    trt: &str,
) -> std::fmt::Result {
    if let Some(id) = obj.instance_id_or_none() {
        let class: StringName = obj.dynamic_class_string();
        debug_string_parts(f, ty, id, class, obj.maybe_refcount(), Some(trt))
    } else {
        write!(f, "{ty} {{ freed obj }}")
    }
}

fn debug_string_parts(
    f: &mut std::fmt::Formatter<'_>,
    ty: &str,
    id: InstanceId,
    class: StringName,
    refcount: Option<usize>,
    trait_name: Option<&str>,
) -> std::fmt::Result {
    let mut builder = f.debug_struct(ty);
    builder
        .field("id", &id.to_i64())
        .field("class", &format_args!("{class}"));

    if let Some(trait_name) = trait_name {
        builder.field("trait", &format_args!("{trait_name}"));
    }

    if let Some(refcount) = refcount {
        builder.field("refc", &refcount);
    }

    builder.finish()
}

pub(crate) fn display_string<T: GodotClass>(
    obj: &Gd<T>,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    let string: GString = obj.raw.as_object_ref().to_string();
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
        "{call_ctx}: instance ID {instance_id} points to a stale, reused object. Please report this to godot-rust maintainers."
    );
}

#[cfg(debug_assertions)]
pub(crate) fn ensure_object_inherits(derived: ClassName, base: ClassName, instance_id: InstanceId) {
    if derived == base
        || base == Object::class_name() // for Object base, anything inherits by definition
        || is_derived_base_cached(derived, base)
    {
        return;
    }

    panic!(
        "Instance of ID {instance_id} has type {derived} but is incorrectly stored in a Gd<{base}>.\n\
        This may happen if you change an object's identity through DerefMut."
    )
}

#[cfg(debug_assertions)]
pub(crate) fn ensure_binding_not_null<T>(binding: sys::GDExtensionClassInstancePtr)
where
    T: GodotClass + Bounds<Declarer = bounds::DeclUser>,
{
    if !binding.is_null() {
        return;
    }

    // Non-tool classes can't be instantiated in the editor.
    if crate::classes::Engine::singleton().is_editor_hint() {
        panic!(
            "Class {} -- null instance; does the class have a Godot creator function? \
            Ensure that the given class is a tool class with #[class(tool)], if it is being accessed in the editor.",
            std::any::type_name::<T>()
        )
    } else {
        panic!(
            "Class {} -- null instance; does the class have a Godot creator function?",
            std::any::type_name::<T>()
        );
    }
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
        ClassDb::singleton().is_parent_class(&derived.to_string_name(), &base.to_string_name());

    // Insert only successful queries. Those that fail are on the error path already and don't need to be fast.
    if is_parent_class {
        cache.insert(key);
    }

    is_parent_class
}
