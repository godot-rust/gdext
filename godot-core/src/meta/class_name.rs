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
// First element (index 0) is always the empty string name, which is used for "no class".
static CLASS_NAMES: Global<Vec<ClassNameEntry>> = Global::new(|| vec![ClassNameEntry::none()]);
static DYNAMIC_INDEX_BY_CLASS_TYPE: Global<HashMap<TypeId, u16>> = Global::default();

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// # Safety
/// Must not use any `ClassName` APIs after this call.
pub unsafe fn cleanup() {
    CLASS_NAMES.lock().clear();
    DYNAMIC_INDEX_BY_CLASS_TYPE.lock().clear();
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

/// Name of a class registered with Godot.
///
/// Holds the Godot name, not the Rust name (they sometimes differ, e.g. Godot `CSGMesh3D` vs Rust `CsgMesh3D`).
///
/// This struct is very cheap to copy. The actual names are cached globally.
///
/// If you need to create your own class name, use [`new_cached()`][Self::new_cached].
///
/// # Ordering
///
/// `ClassName`s are **not** ordered lexicographically, and the ordering relation is **not** stable across multiple runs of your
/// application. When lexicographical order is needed, it's possible to convert this type to [`GString`] or [`String`].
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct ClassName {
    global_index: u16,
}

impl ClassName {
    /// Construct a new class name.
    ///
    /// This is expensive the first time it called for a given `T`, but will be cached for subsequent calls.
    ///
    /// It is not specified when exactly `init_fn` is invoked. However, it must return the same value for the same `T`. Generally, we expect
    /// to keep the invocations limited, so you can use more expensive construction in the closure.
    ///
    /// # Panics
    /// If the string is not ASCII and the Godot version is older than 4.4. From Godot 4.4 onwards, class names can be Unicode.
    pub fn new_cached<T: GodotClass>(init_fn: impl FnOnce() -> String) -> Self {
        // Check if class name exists.
        let type_id = TypeId::of::<T>();
        let mut map = DYNAMIC_INDEX_BY_CLASS_TYPE.lock();

        // Insert into linear vector. Note: this doesn't check for overlaps of TypeId between static and dynamic class names.
        let global_index = *map.entry(type_id).or_insert_with(|| {
            let name = init_fn();

            #[cfg(before_api = "4.4")]
            assert!(
                name.is_ascii(),
                "In Godot < 4.4, class name must be ASCII: '{name}'"
            );

            insert_class(ClassNameSource::Owned(name))
        });

        ClassName { global_index }
    }

    #[doc(hidden)]
    pub fn none() -> Self {
        // First element is always the empty string name.
        Self { global_index: 0 }
    }

    #[doc(hidden)]
    pub fn alloc_next_ascii(class_name_cstr: &'static CStr) -> Self {
        let utf8 = class_name_cstr
            .to_str()
            .expect("class name is invalid UTF-8");

        assert!(
            utf8.is_ascii(),
            "ClassName::alloc_next_ascii() with non-ASCII Unicode string '{utf8}'"
        );

        let global_index = insert_class(ClassNameSource::Borrowed(class_name_cstr));

        Self { global_index }
    }

    #[doc(hidden)]
    pub fn alloc_next_unicode(class_name_str: &'static str) -> Self {
        assert!(
            cfg!(since_api = "4.4"),
            "Before Godot 4.4, class names must be ASCII, but '{class_name_str}' is not.\nSee https://github.com/godotengine/godot/pull/96501."
        );

        assert!(
            !class_name_str.is_ascii(),
            "ClassName::alloc_next_unicode() with ASCII string '{class_name_str}'"
        );

        // StringNames use optimized 1-byte-per-char layout for Latin-1/ASCII, so Unicode can as well use the regular constructor.
        let global_index = insert_class(ClassNameSource::Owned(class_name_str.to_owned()));

        Self { global_index }
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
        let cached_names = CLASS_NAMES.lock();
        let entry = &cached_names[self.global_index as usize];

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
        let cached_names = CLASS_NAMES.lock();
        let entry = &cached_names[self.global_index as usize];

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

/// Adds a new class name to the cache, returning its index.
fn insert_class(name: ClassNameSource) -> u16 {
    let mut names = CLASS_NAMES.lock();
    let index = names
        .len()
        .try_into()
        .expect("Currently limited to 65536 class names");

    names.push(ClassNameEntry::new(name));
    index
}

fn ascii_cstr_to_str(cstr: &CStr) -> &str {
    cstr.to_str().expect("should be validated ASCII")
}
