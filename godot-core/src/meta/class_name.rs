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
use std::ffi::CStr;
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
static CLASS_NAME_CACHE: Global<ClassNameCache> = Global::new(ClassNameCache::new);

/// # Safety
/// Must not use any `ClassName` APIs after this call.
pub unsafe fn cleanup() {
    CLASS_NAME_CACHE.lock().clear();
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Name of a class registered with Godot.
///
/// Holds the Godot name, not the Rust name (they sometimes differ, e.g. Godot `CSGMesh3D` vs Rust `CsgMesh3D`).
///
/// This struct implements `Copy` and is very cheap to copy. The actual names are cached globally.
///
/// You can access existing classes' name using [`GodotClass::class_name()`][crate::obj::GodotClass::class_name].
/// If you need to create your own class name, use [`new_cached()`][Self::new_cached].
///
/// # Ordering
///
/// `ClassName`s are **not** ordered lexicographically, and the ordering relation is **not** stable across multiple runs of your
/// application. When lexicographical order is needed, it's possible to convert this type to [`GString`] or [`String`]. Note that
/// [`StringName`] does not implement `Ord`, and its Godot comparison operators are not lexicographical either.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ClassName {
    global_index: u16,
}

impl ClassName {
    /// Construct a new class name.
    ///
    /// You should typically only need this when implementing `GodotClass` manually, without `#[derive(GodotClass)]`, and overriding
    /// `class_name()`. To access an existing type's class name, use [`<T as GodotClass>::class_name()`][crate::obj::GodotClass::class_name].
    ///
    /// This function is expensive the first time it called for a given `T`, but will be cached for subsequent calls. It can make sense to
    /// store the result in a `static`, to further reduce lookup times, but it's not required.
    ///
    /// We discourage calling this function from different places for the same `T`. But if you do so, `init_fn` must return the same string.
    ///
    /// # Panics
    /// If the string is not ASCII and the Godot version is older than 4.4. From Godot 4.4 onwards, class names can be Unicode.
    pub fn new_cached<T: GodotClass>(init_fn: impl FnOnce() -> String) -> Self {
        Self::new_cached_inner::<T>(init_fn)
    }

    // Without bounds.
    fn new_cached_inner<T: 'static>(init_fn: impl FnOnce() -> String) -> ClassName {
        let type_id = TypeId::of::<T>();
        let mut cache = CLASS_NAME_CACHE.lock();

        // Check if already cached by type
        if let Some(global_index) = cache.get_by_type_id(type_id) {
            return ClassName { global_index };
        }

        // Not cached, need to get or create entry
        let name = init_fn();

        #[cfg(before_api = "4.4")]
        assert!(
            name.is_ascii(),
            "In Godot < 4.4, class name must be ASCII: '{name}'"
        );

        cache.insert_class_name(ClassNameSource::Owned(name), Some(type_id), false)
    }

    /// Create a ClassName from a runtime string (for dynamic class names).
    ///
    /// Will reuse existing `ClassName` entries if the string is recognized.
    // Deliberately not public.
    #[allow(dead_code)] // until used.
    pub(crate) fn new_dynamic(class_name: String) -> Self {
        let mut cache = CLASS_NAME_CACHE.lock();

        cache.insert_class_name(ClassNameSource::Owned(class_name), None, false)
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

    #[doc(hidden)]
    pub fn none() -> Self {
        // First element is always the empty string name.
        Self { global_index: 0 }
    }

    /// Create a new ASCII; expect to be unique. Internal, reserved for macros.
    #[doc(hidden)]
    pub fn __alloc_next_ascii(class_name_cstr: &'static CStr) -> Self {
        let utf8 = class_name_cstr
            .to_str()
            .expect("class name is invalid UTF-8");

        assert!(
            utf8.is_ascii(),
            "ClassName::alloc_next_ascii() with non-ASCII Unicode string '{utf8}'"
        );

        let source = ClassNameSource::Borrowed(class_name_cstr);
        let mut cache = CLASS_NAME_CACHE.lock();
        cache.insert_class_name(source, None, true)
    }

    /// Create a new Unicode entry; expect to be unique. Internal, reserved for macros.
    #[doc(hidden)]
    pub fn __alloc_next_unicode(class_name_str: &'static str) -> Self {
        assert!(
            cfg!(since_api = "4.4"),
            "Before Godot 4.4, class names must be ASCII, but '{class_name_str}' is not.\nSee https://github.com/godotengine/godot/pull/96501."
        );

        assert!(
            !class_name_str.is_ascii(),
            "ClassName::__alloc_next_unicode() with ASCII string '{class_name_str}'"
        );

        let source = ClassNameSource::Owned(class_name_str.to_owned());
        let mut cache = CLASS_NAME_CACHE.lock();
        cache.insert_class_name(source, None, true)
    }

    #[doc(hidden)]
    pub fn is_none(&self) -> bool {
        self.global_index == 0
    }
    //
    // /// Returns the class name as a string slice with static storage duration.
    // pub fn as_str(&self) -> &'static str {
    //     // unwrap() safe, checked in constructor
    //     self.c_str.to_str().unwrap()
    // }

    /// Converts the class name to a `GString`.
    pub fn to_gstring(&self) -> GString {
        self.with_string_name(|s| s.into())
    }

    /// Converts the class name to a `StringName`.
    pub fn to_string_name(&self) -> StringName {
        self.with_string_name(|s| s.clone())
    }

    /// Returns an owned or borrowed `str`.
    pub fn to_cow_str(&self) -> Cow<'static, str> {
        let cache = CLASS_NAME_CACHE.lock();
        let entry = cache.get_entry(self.global_index as usize);
        entry.rust_str.as_cow_str()
    }

    /// The returned pointer is valid indefinitely, as entries are never deleted from the cache.
    /// Since we use `Box<StringName>`, `HashMap` reallocations don't affect the validity of the StringName.
    #[doc(hidden)]
    pub fn string_sys(&self) -> sys::GDExtensionConstStringNamePtr {
        self.with_string_name(|s| s.string_sys())
    }

    // Takes a closure because the mutex guard protects the reference; so the &StringName cannot leave the scope.
    fn with_string_name<R>(&self, func: impl FnOnce(&StringName) -> R) -> R {
        let cache = CLASS_NAME_CACHE.lock();
        let entry = cache.get_entry(self.global_index as usize);

        let string_name = entry
            .godot_str
            .get_or_init(|| entry.rust_str.to_string_name());

        func(string_name)
    }
}

impl fmt::Display for ClassName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.with_string_name(|s| s.fmt(f))
    }
}

impl fmt::Debug for ClassName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let cache = CLASS_NAME_CACHE.lock();
        let entry = cache.get_entry(self.global_index as usize);
        let name = entry.rust_str.as_cow_str();

        if name.is_empty() {
            write!(f, "ClassName(none)")
        } else {
            write!(f, "ClassName({:?})", name)
        }
    }
}

fn ascii_cstr_to_str(cstr: &CStr) -> &str {
    cstr.to_str().expect("should be validated ASCII")
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Entry in the class name cache.
///
/// `StringName` needs to be lazy-initialized because the Godot binding may not be initialized yet.
struct ClassNameEntry {
    rust_str: ClassNameSource,
    godot_str: OnceCell<StringName>,
}

impl ClassNameEntry {
    fn new(rust_str: ClassNameSource) -> Self {
        Self {
            rust_str,
            godot_str: OnceCell::new(),
        }
    }

    fn none() -> Self {
        Self::new(ClassNameSource::Borrowed(c""))
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// `Cow`-like enum for class names, but with C strings as the borrowed variant.
enum ClassNameSource {
    Owned(String),
    Borrowed(&'static CStr),
}

impl ClassNameSource {
    fn to_string_name(&self) -> StringName {
        match self {
            ClassNameSource::Owned(s) => StringName::from(s),
            ClassNameSource::Borrowed(cstr) => StringName::from(*cstr),
        }
    }

    fn as_cow_str(&self) -> Cow<'static, str> {
        match self {
            ClassNameSource::Owned(s) => Cow::Owned(s.clone()),
            ClassNameSource::Borrowed(cstr) => Cow::Borrowed(ascii_cstr_to_str(cstr)),
        }
    }
}
// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Unified cache for all class name data.
struct ClassNameCache {
    /// All class name entries, with index representing [`ClassName::global_index`].
    /// First element (index 0) is always the empty string name, which is used for "no class".
    entries: Vec<ClassNameEntry>,
    /// Cache for type-based lookups.
    type_to_index: HashMap<TypeId, u16>,
    /// Cache for runtime string-based lookups.
    string_to_index: HashMap<String, u16>,
}

impl ClassNameCache {
    fn new() -> Self {
        let mut string_to_index = HashMap::new();
        // Pre-populate string cache with the empty string at index 0.
        string_to_index.insert(String::new(), 0);

        Self {
            entries: vec![ClassNameEntry::none()],
            type_to_index: HashMap::new(),
            string_to_index,
        }
    }

    /// Looks up entries and if not present, inserts them.
    ///
    /// Returns the `ClassName` for the given name.
    ///
    /// # Panics (Debug)
    /// If `expect_first` is true and the string is already present in the cache.
    fn insert_class_name(
        &mut self,
        source: ClassNameSource,
        type_id: Option<TypeId>,
        expect_first: bool,
    ) -> ClassName {
        let name_str = source.as_cow_str();

        if expect_first {
            // Debug verification that we're indeed the first to register this string.
            #[cfg(debug_assertions)]
            assert!(
                !self.string_to_index.contains_key(name_str.as_ref()),
                "insert_class_name() called for already-existing string: {}",
                name_str
            );
        } else {
            // Check string cache first (dynamic path may reuse existing entries).
            if let Some(&existing_index) = self.string_to_index.get(name_str.as_ref()) {
                // Update type cache if we have a TypeId and it's not already cached (dynamic-then-static case).
                // Note: if type_id is Some, we know it came from new_cached_inner after a failed TypeId lookup.
                if let Some(type_id) = type_id {
                    self.type_to_index.entry(type_id).or_insert(existing_index);
                }
                return ClassName {
                    global_index: existing_index,
                };
            }
        }

        // Not found or static path - create new entry.
        let global_index = self.entries.len().try_into().unwrap_or_else(|_| {
            panic!("ClassName cache exceeded maximum capacity of 65536 entries")
        });

        self.entries.push(ClassNameEntry::new(source));
        self.string_to_index
            .insert(name_str.into_owned(), global_index);

        if let Some(type_id) = type_id {
            self.type_to_index.insert(type_id, global_index);
        }

        ClassName { global_index }
    }

    fn get_by_type_id(&self, type_id: TypeId) -> Option<u16> {
        self.type_to_index.get(&type_id).copied()
    }

    fn get_entry(&self, index: usize) -> &ClassNameEntry {
        &self.entries[index]
    }

    fn clear(&mut self) {
        self.entries.clear();
        self.type_to_index.clear();
        self.string_to_index.clear();
    }
}
