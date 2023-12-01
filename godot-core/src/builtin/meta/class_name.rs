/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot_ffi as sys;
use std::collections::HashMap;
use std::ffi::CStr;
use std::{fmt, sync};

use std::hash::{Hash, Hasher};

use crate::builtin::*;

// Why is this so ugly?
// - Mutex: needed for global access (Sync).
// - Option: needed to initialize lazily, because HashMap::new() is not const.
// - Box: needed for pointer stability (HashMap insertion may invalidate pointers -- with_capacity() would be an alternative,
//   but we don't know how many classes).
// In theory a static mut would do the job, however if we allow for manual class registration (at any time), we need to count with
// later adjustments.
// We may also consider OnceLock with a static per class, but that needs to be code-generated (for #[derive] and engine classes), and
// any manually registered classes would need to replicate it later.
static CACHED_STRING_NAMES: sync::Mutex<Option<HashMap<ClassName, Box<StringName>>>> =
    sync::Mutex::new(None);

/// Name of a class registered with Godot.
///
/// Holds the Godot name, not the Rust name (they sometimes differ, e.g. Godot `CSGMesh3D` vs Rust `CsgMesh3D`).
///
/// You cannot construct instances of this type yourself; use [`GodotClass::class_name()`](crate::obj::GodotClass::class_name()).
/// This struct is very cheap to copy.
#[derive(Copy, Clone, Debug)]
pub struct ClassName {
    // Could use small-array optimization for common string lengths.
    c_str: &'static CStr,
    // Possible optimization: could store pre-computed hash. Would need a custom S parameter for HashMap<K, V, S>, see
    // https://doc.rust-lang.org/std/hash/trait.BuildHasher.html. (The default hasher recomputes the hash repeatedly).
}

impl ClassName {
    #[doc(hidden)]
    pub fn from_ascii_cstr(bytes: &'static [u8]) -> Self {
        assert!(bytes.is_ascii(), "string must be ASCII"); // only half of u8 range
        let c_str = CStr::from_bytes_with_nul(bytes).expect("string must be null-terminated");

        Self { c_str }
    }

    #[doc(hidden)]
    pub fn none() -> Self {
        // In Godot, an empty class name means "no class".
        Self::from_ascii_cstr(b"\0")
    }

    /// Returns the class name as a string slice with static storage duration.
    pub fn as_str(&self) -> &'static str {
        // unwrap() safe, checked in constructor
        self.c_str.to_str().unwrap()
    }

    /// Converts the class name to a GString.
    pub fn to_godot_string(&self) -> GString {
        self.with_string_name(|s| s.into())
    }

    /// Converts the class name to a StringName.
    pub fn to_string_name(&self) -> StringName {
        self.with_string_name(|s| s.clone())
    }

    /// The returned pointer is valid indefinitely, as entries are never deleted from the cache.
    /// Since we use Box<StringName>, HashMap reallocations don't affect the validity of the StringName.
    #[doc(hidden)]
    pub fn string_sys(&self) -> sys::GDExtensionStringNamePtr {
        self.with_string_name(|s| s.string_sys())
    }

    // Takes a closure because the mutex guard protects the reference; so the &StringName cannot leave the scope.
    fn with_string_name<R>(&self, func: impl FnOnce(&StringName) -> R) -> R {
        let mut guard = CACHED_STRING_NAMES.lock().unwrap();
        let map = guard.get_or_insert_with(HashMap::new);

        let value = map
            .entry(*self)
            .or_insert_with(|| Box::new(self.load_string_name()));

        func(value)
    }

    fn load_string_name(&self) -> StringName {
        StringName::from(self.c_str.to_str().unwrap())
    }
}

impl PartialEq for ClassName {
    fn eq(&self, other: &Self) -> bool {
        self.c_str == other.c_str
    }
}

impl Eq for ClassName {}

impl Hash for ClassName {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.c_str.hash(state)
    }
}

impl fmt::Display for ClassName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_str().fmt(f)
    }
}
