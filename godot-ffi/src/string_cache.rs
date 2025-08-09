/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::collections::HashMap;
use std::mem::MaybeUninit;
use std::ptr;

use crate as sys;

/// Caches `StringName` instances at initialization.
pub struct StringCache<'a> {
    // Box is needed for element stability (new insertions don't move object; i.e. pointers to it remain valid).
    instances_by_str: HashMap<&'static str, Box<sys::types::OpaqueStringName>>,
    interface: &'a sys::GDExtensionInterface,
    builtin_lifecycle: &'a sys::BuiltinLifecycleTable,
}

impl<'a> StringCache<'a> {
    pub fn new(
        interface: &'a sys::GDExtensionInterface,
        builtin_lifecycle: &'a sys::BuiltinLifecycleTable,
    ) -> Self {
        Self {
            instances_by_str: HashMap::new(),
            interface,
            builtin_lifecycle,
        }
    }

    /// Get a pointer to a `StringName`. Reuses cached instances, only deallocates on destruction of this cache.
    pub fn fetch(&mut self, key: &'static str) -> sys::GDExtensionStringNamePtr {
        assert!(key.is_ascii(), "string is not ASCII: {key}");

        // Already cached.
        if let Some(opaque_box) = self.instances_by_str.get_mut(key) {
            return box_to_sname_ptr(opaque_box);
        }

        let mut sname = MaybeUninit::<sys::types::OpaqueStringName>::uninit();
        let sname_ptr = sname.as_mut_ptr();

        // For Godot 4.1, construct StringName via String + conversion.
        #[cfg(before_api = "4.2")]
        unsafe {
            let string_new_with_latin1_chars_and_len = self
                .interface
                .string_new_with_latin1_chars_and_len
                .unwrap_unchecked();
            let string_name_from_string = self.builtin_lifecycle.string_name_from_string;
            let string_destroy = self.builtin_lifecycle.string_destroy;

            let mut string = MaybeUninit::<sys::types::OpaqueString>::uninit();
            let string_ptr = string.as_mut_ptr();

            // Construct String.
            string_new_with_latin1_chars_and_len(
                string_uninit_ptr(string_ptr),
                key.as_ptr() as *const std::os::raw::c_char,
                key.len() as sys::GDExtensionInt,
            );

            // Convert String -> StringName.
            string_name_from_string(
                sname_uninit_type_ptr(sname_ptr),
                [sys::to_const_ptr(string_type_ptr(string_ptr))].as_ptr(),
            );

            // Destroy String.
            string_destroy(string_type_ptr(string_ptr));
        }

        // For Godot 4.2+, construct StringName directly from C string.
        #[cfg(since_api = "4.2")]
        unsafe {
            let string_name_new_with_utf8_chars_and_len = self
                .interface
                .string_name_new_with_utf8_chars_and_len
                .unwrap_unchecked();

            // Construct StringName from string (non-static, we only need them during the cache's lifetime).
            // There is no _latin_*() variant that takes length, so we have to use _utf8_*() instead.
            string_name_new_with_utf8_chars_and_len(
                sname_uninit_ptr(sname_ptr),
                key.as_ptr() as *const std::os::raw::c_char,
                key.len() as sys::GDExtensionInt,
            );
        }

        // Return StringName.
        let opaque = unsafe { sname.assume_init() };

        let mut opaque_box = Box::new(opaque);
        let sname_ptr = box_to_sname_ptr(&mut opaque_box);

        self.instances_by_str.insert(key, opaque_box);
        sname_ptr
    }
}

/// Destroy all string names.
impl Drop for StringCache<'_> {
    fn drop(&mut self) {
        let string_name_destroy = self.builtin_lifecycle.string_name_destroy;

        unsafe {
            for (_, mut opaque_box) in self.instances_by_str.drain() {
                let opaque_ptr = ptr::addr_of_mut!(*opaque_box);
                string_name_destroy(sname_type_ptr(opaque_ptr));
            }
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Implementation
// These are tiny wrappers to avoid exposed `as` casts (which are very easy to get wrong, i.e. extra dereference).
// Using a trait to abstract over String/StringName is overkill and also doesn't work due to both having same Opaque<N> type.

fn box_to_sname_ptr(
    boxed: &mut Box<sys::types::OpaqueStringName>,
) -> sys::GDExtensionStringNamePtr {
    let opaque_ptr = ptr::addr_of_mut!(**boxed);
    opaque_ptr as sys::GDExtensionStringNamePtr
}

#[cfg(before_api = "4.2")]
unsafe fn string_type_ptr(opaque_ptr: *mut sys::types::OpaqueString) -> sys::GDExtensionTypePtr {
    opaque_ptr as sys::GDExtensionTypePtr
}

#[cfg(before_api = "4.2")]
unsafe fn string_uninit_ptr(
    opaque_ptr: *mut sys::types::OpaqueString,
) -> sys::GDExtensionUninitializedStringPtr {
    opaque_ptr as sys::GDExtensionUninitializedStringPtr
}

#[cfg(since_api = "4.2")]
unsafe fn sname_uninit_ptr(
    opaque_ptr: *mut sys::types::OpaqueStringName,
) -> sys::GDExtensionUninitializedStringNamePtr {
    opaque_ptr as sys::GDExtensionUninitializedStringNamePtr
}

unsafe fn sname_type_ptr(opaque_ptr: *mut sys::types::OpaqueStringName) -> sys::GDExtensionTypePtr {
    opaque_ptr as sys::GDExtensionTypePtr
}

#[cfg(before_api = "4.2")]
unsafe fn sname_uninit_type_ptr(
    opaque_ptr: *mut sys::types::OpaqueStringName,
) -> sys::GDExtensionUninitializedTypePtr {
    opaque_ptr as sys::GDExtensionUninitializedTypePtr
}
