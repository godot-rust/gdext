/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Legacy 4.0 API
//!
//! The old API uses a struct `GDExtensionInterface`, which is passed to the extension entry point (via pointer).
//! This struct contains function pointers to all FFI functions defined in the `gdextension_interface.h` header.

use crate as sys;
use crate::compat::BindingCompat;

pub type InitCompat = *const sys::GDExtensionInterface;

impl BindingCompat for *const sys::GDExtensionInterface {
    fn ensure_static_runtime_compatibility(&self) {
        // We try to read the first fields of the GDExtensionInterface struct, which are version numbers.
        // If those are unrealistic numbers, chances are high that `self` is in fact a function pointer (used for Godot 4.1.x).
        let data_ptr = *self;

        // We cannot print runtime version. We _could_ theoretically fetch the `get_godot_version` function pointer through `get_proc_address`,
        // but that's not adding that much information. The Godot engine already prints its version on startup.
        let static_version = crate::GdextBuild::godot_static_version_string();
        assert!(
            // SAFETY: None. Reading a function pointer as data is UB.
            // However, the alternative is to run into even harder UB because we happily interpret the pointer as *const GDExtensionInterface.
            // So, this is a best-effort and "works in practice" heuristic to warn the user when running a 4.0.x extension under 4.1+.
            // If comparing the first field already fails, we don't even need to read the 2nd field.
            unsafe { data_ptr.read().version_major } == 4
            && unsafe { data_ptr.read().version_minor } == 0,

            "gdext was compiled against a legacy Godot version ({static_version}),\n\
            but initialized by a newer Godot binary (4.1+).\n\
            \n\
            This setup is not supported. Please recompile the Rust extension with a newer Godot version\n\
            (or run it with an older Godot version).\n"
        );
    }

    fn runtime_version(&self) -> sys::GDExtensionGodotVersion {
        // SAFETY: this method is only invoked after the static compatibility check has passed.
        // We thus know that Godot 4.0.x runs, and *self is a GDExtensionInterface pointer.
        let interface = unsafe { &**self };
        sys::GDExtensionGodotVersion {
            major: interface.version_major,
            minor: interface.version_minor,
            patch: interface.version_patch,
            string: interface.version_string,
        }
    }

    fn load_interface(&self) -> sys::GDExtensionInterface {
        unsafe { **self }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Polyfill for types referenced in function pointer tables

pub(crate) type GDExtensionInterfaceVariantGetPtrBuiltinMethod = Option<
    unsafe extern "C" fn(
        p_type: crate::GDExtensionVariantType,
        p_method: crate::GDExtensionConstStringNamePtr,
        p_hash: crate::GDExtensionInt,
    ) -> crate::GDExtensionPtrBuiltInMethod,
>;

pub(crate) type GDExtensionInterfaceClassdbGetMethodBind = Option<
    unsafe extern "C" fn(
        p_classname: crate::GDExtensionConstStringNamePtr,
        p_methodname: crate::GDExtensionConstStringNamePtr,
        p_hash: crate::GDExtensionInt,
    ) -> crate::GDExtensionMethodBindPtr,
>;
