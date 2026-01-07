/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::any::TypeId;
use std::borrow::Cow;
use std::cell::OnceCell;
use std::collections::HashMap;
use std::fmt;
use std::hash::Hash;

use godot_ffi as sys;
use sys::Global;

use crate::builtin::*;
use crate::obj::GodotClass;

// Alternative optimizations:
// - Small-array optimization for common string lengths.
// - Use HashMap and store pre-computed hash. Would need a custom S parameter for HashMap<K, V, S>, see
//   https://doc.rust-lang.org/std/hash/trait.BuildHasher.html (the default hasher recomputes the hash repeatedly).
//

/// Global cache of class names.
static CLASS_ID_CACHE: Global<ClassIdCache> = Global::new(ClassIdCache::new);

/// # Safety
/// Must not use any `ClassId` APIs after this call.
pub unsafe fn cleanup() {
    CLASS_ID_CACHE.lock().clear();
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Globally unique ID of a class registered with Godot.
///
/// This struct implements `Copy` and is very cheap to copy and compare with other `ClassId`s.
///
/// `ClassId` can also be used to obtain the class name, which is cached globally, not per-instance. Note that it holds the Godot name,
/// not the Rust name -- they sometimes differ, e.g. Godot `CSGMesh3D` vs Rust `CsgMesh3D`.
///
/// You can access existing classes' ID using [`GodotClass::class_id()`][crate::obj::GodotClass::class_id].
/// If you need to create your own class ID, use [`new_cached()`][Self::new_cached].
///
/// # Ordering
///
/// `ClassId`s are **not** ordered lexicographically, and the ordering relation is **not** stable across multiple runs of your
/// application. When lexicographical order is needed, it's possible to convert this type to [`GString`] or [`String`]. Note that
/// [`StringName`] does not implement `Ord`, and its Godot comparison operators are not lexicographical either.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ClassId {
    global_index: u16,
}

impl ClassId {
    /// Construct a new class name.
    ///
    /// You should typically only need this when implementing `GodotClass` manually, without `#[derive(GodotClass)]`, and overriding
    /// `class_id()`. To access an existing type's class name, use [`<T as GodotClass>::class_id()`][crate::obj::GodotClass::class_id].
    ///
    /// This function is expensive the first time it called for a given `T`, but will be cached for subsequent calls. It can make sense to
    /// store the result in a `static`, to further reduce lookup times, but it's not required.
    ///
    /// We discourage calling this function from different places for the same `T`. But if you do so, `init_fn` must return the same string.
    ///
    /// # Panics
    /// If the string contains non-ASCII characters and the Godot version is older than 4.4. From Godot 4.4 onwards, class names can be Unicode;
    /// See <https://github.com/godotengine/godot/pull/96501>.
    pub fn new_cached<T: GodotClass>(init_fn: impl FnOnce() -> String) -> Self {
        Self::new_cached_inner::<T>(init_fn)
    }

    // Without bounds.
    fn new_cached_inner<T: 'static>(init_fn: impl FnOnce() -> String) -> ClassId {
        let type_id = TypeId::of::<T>();
        let mut cache = CLASS_ID_CACHE.lock();

        // Check if already cached by type
        if let Some(global_index) = cache.get_by_type_id(type_id) {
            return ClassId { global_index };
        }

        // Not cached, need to get or create entry
        let name = init_fn();

        #[cfg(before_api = "4.4")]
        assert!(
            name.is_ascii(),
            "In Godot < 4.4, class name must be ASCII: '{name}'"
        );

        cache.insert_class_id(Cow::Owned(name), Some(type_id), false)
    }

    /// Create a `ClassId` from a class name only known at runtime.
    ///
    /// Unlike [`ClassId::new_cached()`], this doesn't require a static type parameter. Useful for classes defined outside Rust code, e.g. in
    /// scripts.
    ///
    /// Multiple calls with the same name return equal `ClassId` instances (but may need a lookup).
    ///
    /// # Example
    /// ```no_run
    /// use godot::meta::ClassId;
    ///
    /// let a = ClassId::new_dynamic("MyGDScriptClass");
    /// let b = ClassId::new_dynamic("MyGDScriptClass");
    /// assert_eq!(a, b);
    /// ```
    ///
    /// # Panics
    /// If the string contains non-ASCII characters and the Godot version is older than 4.4. From Godot 4.4 onwards, class names can be Unicode;
    /// See <https://github.com/godotengine/godot/pull/96501>.
    pub fn new_dynamic(class_name: impl Into<CowStr>) -> Self {
        let mut cache = CLASS_ID_CACHE.lock();

        cache.insert_class_id(class_name.into(), None, false)
    }

    // Test-only APIs.
    #[cfg(feature = "trace")] // itest only.
    #[doc(hidden)]
    pub fn __cached<T: 'static>(init_fn: impl FnOnce() -> String) -> Self {
        Self::new_cached_inner::<T>(init_fn)
    }

    #[cfg(feature = "trace")] // itest only.
    #[doc(hidden)]
    pub fn __dynamic(class_name: &str) -> Self {
        Self::new_dynamic(class_name.to_string())
    }

    /// Returns a `ClassId` representing "no class" (empty class name) for non-object property types.
    ///
    /// This is used for properties that don't have an associated class, e.g. built-in types like `i32`, `GString`, `Vector3` etc.
    /// When constructing a [`PropertyInfo`](crate::meta::PropertyInfo) for non-class types, you can use `ClassId::none()` for the `class_id` field.
    pub fn none() -> Self {
        // First element is always the empty string name.
        Self { global_index: 0 }
    }

    /// Create a new Unicode entry; expect to be unique. Internal, reserved for macros.
    #[doc(hidden)]
    pub fn __alloc_next_unicode(class_name_str: &'static str) -> Self {
        #[cfg(before_api = "4.4")]
        assert!(
            class_name_str.is_ascii(),
            "Before Godot 4.4, class names must be ASCII, but '{class_name_str}' is not.\nSee https://github.com/godotengine/godot/pull/96501."
        );

        let source = Cow::Borrowed(class_name_str);
        let mut cache = CLASS_ID_CACHE.lock();
        cache.insert_class_id(source, None, true)
    }

    #[doc(hidden)]
    pub fn is_none(&self) -> bool {
        self.global_index == 0
    }

    /// Returns the class name as a `GString`.
    pub fn to_gstring(&self) -> GString {
        self.with_string_name(|s| s.into())
    }

    /// Returns the class name as a `StringName`.
    pub fn to_string_name(&self) -> StringName {
        self.with_string_name(|s| s.clone())
    }

    /// Returns an owned or borrowed `str` representing the class name.
    pub fn to_cow_str(&self) -> CowStr {
        let cache = CLASS_ID_CACHE.lock();
        let entry = cache.get_entry(self.global_index as usize);
        entry.rust_str.clone()
    }

    /// The returned pointer is valid indefinitely, as entries are never deleted from the cache.
    /// Since we use `Box<StringName>`, `HashMap` reallocations don't affect the validity of the StringName.
    #[doc(hidden)]
    pub fn string_sys(&self) -> sys::GDExtensionConstStringNamePtr {
        self.with_string_name(|s| s.string_sys())
    }

    // Takes a closure because the mutex guard protects the reference; so the &StringName cannot leave the scope.
    fn with_string_name<R>(&self, func: impl FnOnce(&StringName) -> R) -> R {
        let cache = CLASS_ID_CACHE.lock();
        let entry = cache.get_entry(self.global_index as usize);

        let string_name = entry
            .godot_str
            .get_or_init(|| StringName::from(entry.rust_str.as_ref()));

        func(string_name)
    }
}

impl fmt::Display for ClassId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.to_cow_str().fmt(f)
    }
}

impl fmt::Debug for ClassId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = self.to_cow_str();

        if name.is_empty() {
            write!(f, "ClassId(none)")
        } else {
            write!(f, "ClassId({:?})", name)
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Entry in the class name cache.
///
/// `StringName` needs to be lazy-initialized because the Godot binding may not be initialized yet.
struct ClassIdEntry {
    rust_str: CowStr,
    godot_str: OnceCell<StringName>,
}

impl ClassIdEntry {
    const fn new(rust_str: CowStr) -> Self {
        Self {
            rust_str,
            godot_str: OnceCell::new(),
        }
    }

    fn none() -> Self {
        Self::new(Cow::Borrowed(""))
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Unified cache for all class name data.
struct ClassIdCache {
    /// All class name entries, with index representing [`ClassId::global_index`].
    /// First element (index 0) is always the empty string name, which is used for "no class".
    entries: Vec<ClassIdEntry>,
    /// Cache for type-based lookups.
    type_to_index: HashMap<TypeId, u16>,
    /// Cache for runtime string-based lookups.
    string_to_index: HashMap<String, u16>,
}

impl ClassIdCache {
    fn new() -> Self {
        let mut string_to_index = HashMap::new();
        // Pre-populate string cache with the empty string at index 0.
        string_to_index.insert(String::new(), 0);

        Self {
            entries: vec![ClassIdEntry::none()],
            type_to_index: HashMap::new(),
            string_to_index,
        }
    }

    /// Looks up entries and if not present, inserts them.
    ///
    /// Returns the `ClassId` for the given name.
    ///
    /// # Panics (safeguards-balanced)
    /// If `expect_first` is true and the string is already present in the cache.
    fn insert_class_id(
        &mut self,
        source: CowStr,
        type_id: Option<TypeId>,
        expect_first: bool,
    ) -> ClassId {
        if expect_first {
            // Debug verification that we're indeed the first to register this string.
            sys::balanced_assert!(
                !self.string_to_index.contains_key(source.as_ref()),
                "insert_class_name() called for already-existing string: {}",
                source
            );
        } else {
            // Check string cache first (dynamic path may reuse existing entries).
            if let Some(&existing_index) = self.string_to_index.get(source.as_ref()) {
                // Update type cache if we have a TypeId and it's not already cached (dynamic-then-static case).
                // Note: if type_id is Some, we know it came from new_cached_inner after a failed TypeId lookup.
                if let Some(type_id) = type_id {
                    self.type_to_index.entry(type_id).or_insert(existing_index);
                }
                return ClassId {
                    global_index: existing_index,
                };
            }
        }

        // Not found or static path - create new entry.
        let global_index =
            self.entries.len().try_into().unwrap_or_else(|_| {
                panic!("ClassId cache exceeded maximum capacity of 65536 entries")
            });

        self.entries.push(ClassIdEntry::new(source.clone()));
        self.string_to_index
            .insert(source.into_owned(), global_index);

        if let Some(type_id) = type_id {
            self.type_to_index.insert(type_id, global_index);
        }

        ClassId { global_index }
    }

    fn get_by_type_id(&self, type_id: TypeId) -> Option<u16> {
        self.type_to_index.get(&type_id).copied()
    }

    fn get_entry(&self, index: usize) -> &ClassIdEntry {
        &self.entries[index]
    }

    fn clear(&mut self) {
        // MACOS-PARTIAL-RELOAD: Previous implementation for when upstream fixes `.gdextension` reload.
        // self.entries.clear();
        // self.type_to_index.clear();
        // self.string_to_index.clear();

        // MACOS-PARTIAL-RELOAD: Preserve existing `ClassId` entries when only the `.gdextension` reloads so indices stay valid.
        // There are two types of hot reload: `dylib` reload (`dylib` `mtime` newer) unloads and reloads the library, whereas
        // `.gdextension` reload (`.gdextension` `mtime` newer) re-initializes the existing `dylib` without unloading it. To handle
        // `.gdextension` reload, keep the backing entries (and thus the `string_to_index` map) but drop cached Godot `StringNames`
        // and the `TypeId` lookup so they can be rebuilt.
        for entry in &mut self.entries {
            entry.godot_str = OnceCell::new();
        }

        self.type_to_index.clear();
    }
}
