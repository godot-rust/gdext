/*
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

        let interface = unsafe { &**self };
        let major = interface.version_major;
        let minor = interface.version_minor;

        // We cannot print version (major/minor are parts of the function pointer). We _could_ theoretically interpret it as
        // GetProcAddr function pointer and call get_godot_version, but that's not adding that much useful information and may
        // also fail.
        let static_version = crate::GdextBuild::godot_static_version_string();
        assert!(major == 4 && minor == 0,
            "gdext was compiled against a legacy Godot version ({static_version}),\n\
            but initialized by a newer Godot binary (4.1+).\n\
            \n\
            You have multiple options:\n\
            1) Recompile gdext against the newer Godot version.\n\
            2) If you want to use a legacy extension under newer Godot, open the .gdextension file\n   \
               and add `compatibility_minimum = 4.0` under the [configuration] section.\n"
        );
    }

    fn runtime_version(&self) -> sys::GDExtensionGodotVersion {
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
