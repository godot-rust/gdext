/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Runtime checks and inspection of Godot classes.

use std::fmt::Write;
use std::sync::atomic::{AtomicPtr, AtomicU64};

use crate::builtin::{GString, StringName, Variant};
use crate::obj::{Bounds, EngineBitfield, Gd, GodotClass, InstanceId, RawGd, bounds};
use crate::{init, sys};

#[cfg(safeguards_strict)]
mod strict {
    pub use crate::builtin::VariantType;
    pub use crate::classes::{ClassDb, Object};
    pub use crate::meta::ClassId;
    pub use crate::obj::Singleton;
}

#[cfg(safeguards_balanced)]
mod balanced {
    pub(crate) use crate::meta::CallContext;
}

#[cfg(safeguards_balanced)]
use balanced::*;
#[cfg(safeguards_strict)]
use strict::*;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Debug/Display support for classes and enums

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
    sys::strict_assert_eq!(obj.get_type(), VariantType::OBJECT);

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
            let count = obj
                .call("get_reference_count", &[])
                .try_to_relaxed::<i32>()
                .expect("get_reference_count() must return integer");

            Ok(count as usize)
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
    sys::strict_assert_eq!(obj.get_type(), VariantType::OBJECT);

    match obj.try_to::<Gd<crate::classes::Object>>() {
        Ok(obj) => {
            let id = obj.instance_id(); // Guaranteed valid, since conversion would have failed otherwise.
            let class = obj.dynamic_class_string();

            // Refcount is off-by-one due to now-created Gd<T> from conversion; correct by -1.
            let refcount = match obj.maybe_refcount() {
                Some(Ok(rc)) => Some(Ok(rc.saturating_sub(1))),
                Some(Err(e)) => Some(Err(e)),
                None => None,
            };

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
    refcount: Option<Result<usize, ()>>,
    trait_name: Option<&str>,
) -> std::fmt::Result {
    let mut builder = f.debug_struct(ty);
    builder
        .field("id", &id.to_i64())
        .field("class", &format_args!("{class}"));

    if let Some(trait_name) = trait_name {
        builder.field("trait", &format_args!("{trait_name}"));
    }

    match refcount {
        Some(Ok(refcount)) => {
            builder.field("refc", &refcount);
        }
        Some(Err(_)) => {
            builder.field("refc", &"(N/A during init or drop)");
        }
        None => {}
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

/// Format bitfield for `Debug` impl.
// Make public doc-hidden in the future, once user bitfields are supported.
pub(crate) fn debug_bitfield<T: EngineBitfield>(
    bitfield: T,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    let value_bits = bitfield.ord();
    let mut remaining_bits = value_bits;
    let mut string = String::new();
    let mut first = true;

    for &c in T::all_constants() {
        let mask = c.value().ord();

        // Include zero bits only if the entire value is zero.
        // Example: if value is 0, then set NONE.
        if mask == 0 && value_bits != 0 {
            continue;
        }

        // Include all bits that are *fully* represented in bitfield's value.
        // Example: NONE(0), FAST(1), GOOD(2), CHEAP(4), DEFAULT(3) = FAST|GOOD.
        //   If value is 3, then include all of FAST|GOOD|DEFAULT.
        if value_bits & mask == mask {
            remaining_bits &= !mask;

            if first {
                first = false;
            } else {
                string.push_str(" | ");
            }
            string.push_str(c.rust_name());
        }
    }

    if remaining_bits != 0 {
        if !first {
            string.push_str(" | ");
        }

        write!(string, "Unknown(0x{remaining_bits:X})")?;
    }

    let bitfield_name = sys::short_type_name::<T>();
    write!(f, "{bitfield_name} {{ {string} }}")
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Object lifetime and validity

pub(crate) fn object_ptr_from_id(instance_id: InstanceId) -> sys::GDExtensionObjectPtr {
    // SAFETY: Godot looks up ID in ObjectDB and returns null if not found.
    unsafe { sys::interface_fn!(object_get_instance_from_id)(instance_id.to_u64()) }
}

pub(crate) fn construct_engine_object<T>() -> Gd<T>
where
    T: GodotClass + Bounds<Declarer = bounds::DeclEngine>,
{
    let mut obj = unsafe {
        let object_ptr = sys::classdb_construct_object(T::class_id().string_sys());
        Gd::<T>::from_constructed_obj_sys(object_ptr)
    };
    #[cfg(since_api = "4.4")]
    obj.upcast_object_mut()
        .notify(crate::classes::notify::ObjectNotification::POSTINITIALIZE);

    obj
}

/// # Safety
/// The caller must ensure that `class_name` corresponds to the actual class name of type `T`.
pub(crate) unsafe fn singleton_unchecked_type<T>(class_name: &StringName) -> Gd<T>
where
    T: GodotClass,
{
    // The pointer from global_get_singleton() is only valid while T's init level is loaded. After it unloads, Godot frees the singleton but keeps
    // a dangling map entry, so dereferencing that pointer would be UB. Validate (balanced+ safeguards). Not a problem _before_ the singleton is
    // loaded: Godot correctly returns null then, which from_obj_sys turns into a panic.
    if let Some(current) = crate::init::current_init_level() {
        sys::balanced_assert!(
            current >= T::INIT_LEVEL,
            "{}::singleton() called after the singleton was unloaded (deinit stage).\n\
            Use `godot::init::is_singleton_available()` to check.",
            std::any::type_name::<T>(),
        );
    }

    // SAFETY: class_name validity upheld by caller; binding is initialized.
    unsafe {
        let object_ptr = sys::interface_fn!(global_get_singleton)(class_name.string_sys());
        Gd::<T>::from_obj_sys(object_ptr)
    }
}

/// Per-singleton cache for engine and user singletons.
///
/// `level == None` means "not cached"; otherwise it holds the level at which `ptr` was fetched and `generation` the deinit generation then. `ptr` is
/// trusted only while the current level still covers `level` and no full deinit happened since (see [`init::singleton_cache_generation`]).
///
/// Ordering: the slow path writes `ptr`/`generation` `Relaxed`, then stores `level` last with `Release`. The fast path reads `level` first with
/// `Acquire`; that pairs with the store, making the earlier `ptr`/`generation` writes visible, so they can be read `Relaxed`.
///
/// Public (doc-hidden) only so that `#[class(singleton)]`-generated code can declare a per-type static; not part of the supported API.
#[doc(hidden)]
pub struct SingletonCache {
    ptr: AtomicPtr<std::ffi::c_void>,
    generation: AtomicU64,
    level: sys::AtomicEnum<Option<init::InitLevel>>,
}

impl SingletonCache {
    // Not Default::default() because of const.
    #[allow(clippy::new_without_default)]
    pub const fn new() -> Self {
        Self {
            ptr: AtomicPtr::new(std::ptr::null_mut()),
            generation: AtomicU64::new(0),
            level: sys::AtomicEnum::default(), // None.
        }
    }
}

/// Cached variant of [`singleton_unchecked_type`], for engine and user (`#[class(singleton)]`) singletons alike.
///
/// A singleton's pointer is stable while its init level is loaded, so the `global_get_singleton()` lookup + `StringName` construction is just
/// overhead. This caches the ptr per type, gated on the init level (and a deinit generation) to structurally prevent stale-pointer use.
///
/// # Safety
/// Same as [`singleton_unchecked_type`]: `make_class_name` must yield `T` class name, and its pointer must be stable during its level lifetime.
pub(crate) unsafe fn cached_singleton<T>(
    cache: &SingletonCache,
    make_class_name: impl FnOnce() -> StringName,
) -> Gd<T>
where
    T: GodotClass,
{
    use std::sync::atomic::Ordering;

    let current = init::current_init_level();
    let generation = init::singleton_cache_generation();

    // Fast path: trust cache if current level still covers cached level and no full deinit since. Read `level` first (`Acquire`), then
    // `ptr`/`generation` `Relaxed` (see SingletonCache docs).
    if let Some(cached_level) = cache.level.load()
        && current.is_some_and(|current| current >= cached_level)
        && cache.generation.load(Ordering::Relaxed) == generation
    {
        let ptr = cache.ptr.load(Ordering::Relaxed);
        // SAFETY: level + generation guard guarantee singleton still alive; from_obj_sys handles ref-count.
        return unsafe { Gd::<T>::from_obj_sys(ptr.cast()) };
    }

    // Slow path. Missing FFI binding makes a call UB -> turn that into a clean panic (covers global ctor/dtor). A `None` level with binding up
    // is the legit early-Core registration window: fall back to an uncached fetch, just don't cache.
    assert!(
        sys::is_godot_initialized(),
        "{}::singleton() called while the Godot FFI binding is unavailable (global init/deinit). \
        See is_singleton_available().",
        std::any::type_name::<T>(),
    );

    // The pointer from global_get_singleton() is only valid while T's init level is loaded. After it unloads, Godot frees the singleton but
    // keeps a dangling map entry, so dereferencing that pointer would be UB. Validate except for safeguards-disengaged. This is not a problem
    // _before_ singleton is loaded; Godot correctly returns null.
    if let Some(current) = current {
        sys::balanced_assert!(
            current >= T::INIT_LEVEL,
            "{}::singleton() called after its init level was unloaded; the singleton no longer exists.\n\
            Use `godot::init::is_singleton_available()` to check.",
            std::any::type_name::<T>(),
        );
    }

    let class_name = make_class_name();
    // SAFETY: class_name matches T; binding initialized (asserted above).
    let object_ptr = unsafe { sys::interface_fn!(global_get_singleton)(class_name.string_sys()) };

    // Cache only a valid ptr with the level it was observed at; null ptr or `None` level => no valid key, don't cache (fetch still returns it).
    if !object_ptr.is_null()
        && let Some(current) = current
    {
        // Write `ptr`/`generation` `Relaxed`, then store `level` last with `Release` (see SingletonCache docs).
        cache.ptr.store(object_ptr.cast(), Ordering::Relaxed);
        cache.generation.store(generation, Ordering::Relaxed);
        cache.level.store(Some(current));
    }

    // SAFETY: null => from_obj_sys panics, identical to current behavior.
    unsafe { Gd::<T>::from_obj_sys(object_ptr) }
}

/// Checks that the object with the given instance ID is still alive and that the pointer is valid.
///
/// This does **not** perform type checking — use `ensure_object_type()` for that.
///
/// # Panics (balanced+strict safeguards)
/// If the object has been freed or the instance ID points to a different object.
#[cfg(safeguards_balanced)]
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

#[cfg(safeguards_strict)]
pub(crate) fn ensure_object_inherits(derived: ClassId, base: ClassId, instance_id: InstanceId) {
    if derived == base
        || base == Object::class_id() // for Object base, anything inherits by definition
        || is_derived_base_cached(derived, base)
    {
        return;
    }

    panic!(
        "Instance of ID {instance_id} has type {derived} but is incorrectly stored in a Gd<{base}>.\n\
        This may happen if you change an object's identity through DerefMut."
    )
}

#[cfg(safeguards_strict)]
pub(crate) fn ensure_binding_not_null<T>(binding: sys::GDExtensionClassInstancePtr)
where
    T: GodotClass + Bounds<Declarer = bounds::DeclUser>,
{
    if !binding.is_null() {
        return;
    }

    // Behavior depending on editor state:
    // * Editor (with `upcoming-editor-placeholders`): class substituted by PlaceholderExtensionInstance; null binding expected -> OK.
    //   Accessing `bind()`/`bind_mut()` on placeholders would still panic, independently of this.
    // * Runtime: null binding is a bug -> panic.
    // * Unknown: Godot < 4.4 before InitLevel::Scene; no placeholders exist that early -> panic.
    let placeholder_ok =
        cfg!(feature = "upcoming-editor-placeholders") && sys::is_editor_or_unknown() == Some(true);
    if !placeholder_ok {
        panic!(
            "Class {} -- null instance; does the class have a Godot creator function?\n\
            If used in the editor, make sure to use #[class(tool)].",
            std::any::type_name::<T>()
        );
    }
}

/// Panic emitted when `bind()` / `bind_mut()` is called on a placeholder instance (runtime class accessed in the editor).
#[track_caller]
pub(crate) fn panic_placeholder_bind<T>(method: &str) -> ! {
    panic!(
        "Gd::{method}() called on a placeholder instance of `{name}`.\n\
        A non-tool class does not have a real instance in the editor.\n\
        Use `#[class(tool)]`, or guard with `init::is_editor_hint()`.",
        name = std::any::type_name::<T>(),
    )
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Implementation of this file

/// Checks if `derived` inherits from `base`, using a cache for _successful_ queries.
#[cfg(safeguards_strict)]
fn is_derived_base_cached(derived: ClassId, base: ClassId) -> bool {
    use std::collections::HashSet;

    use sys::Global;

    static CACHE: Global<HashSet<(ClassId, ClassId)>> = Global::default();

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
