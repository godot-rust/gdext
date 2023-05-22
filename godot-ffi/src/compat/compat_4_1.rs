/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Modern 4.1+ API
//!
//! The extension entry point is passed `get_proc_address` function pointer, which can be used to load all other
//! GDExtension FFI functions dynamically. This is a departure from the previous struct-based approach.
//!
//! Relevant upstream PR: https://github.com/godotengine/godot/pull/76406

use crate as sys;
use crate::compat::BindingCompat;

pub type InitCompat = sys::GDExtensionInterfaceGetProcAddress;

#[repr(C)]
struct LegacyLayout {
    version_major: u32,
    version_minor: u32,
    version_patch: u32,
    version_string: *const std::ffi::c_char,
}

impl BindingCompat for sys::GDExtensionInterfaceGetProcAddress {
    fn ensure_static_runtime_compatibility(&self) {
        // In Godot 4.0.x, before the new GetProcAddress mechanism, the init function looked as follows.
        // In place of the `get_proc_address` function pointer, the `p_interface` data pointer was passed.
        //
        // typedef GDExtensionBool (*GDExtensionInitializationFunction)(
        //     const GDExtensionInterface *p_interface,
        //     GDExtensionClassLibraryPtr p_library,
        //     GDExtensionInitialization *r_initialization
        // );
        //
        // Also, the GDExtensionInterface struct was beginning with these fields:
        //
        // typedef struct {
        //     uint32_t version_major;
        //     uint32_t version_minor;
        //     uint32_t version_patch;
        //     const char *version_string;
        //     ...
        // } GDExtensionInterface;
        //
        // As a result, we can try to interpret the function pointer as a legacy GDExtensionInterface data pointer and check if the
        // first fields have values version_major=4 and version_minor=0. This might be deep in UB territory, but the alternative is
        // to not be able to detect Godot 4.0.x at all, and run into UB anyway.

        let get_proc_address = self.expect("get_proc_address unexpectedly null");
        let data_ptr = get_proc_address as *const LegacyLayout; // crowbar it via `as` cast

        // Assumption is that we have at least 8 bytes of memory to safely read from (for both the data and the function case).
        let major = unsafe { data_ptr.read().version_major };
        let minor = unsafe { data_ptr.read().version_minor };
        let patch = unsafe { data_ptr.read().version_patch };

        if major != 4 || minor != 0 {
            // Technically, major should always be 4; loading Godot 3 will crash anyway.
            return;
        }

        let static_version = crate::GdextBuild::godot_static_version_string();
        let runtime_version = unsafe {
            let char_ptr = data_ptr.read().version_string;
            let c_str = std::ffi::CStr::from_ptr(char_ptr);

            String::from_utf8_lossy(c_str.to_bytes())
                .as_ref()
                .strip_prefix("Godot Engine ")
                .unwrap_or(&String::from_utf8_lossy(c_str.to_bytes()))
                .to_string()
        };

        // Version 4.0.999 is used to signal that we're running Godot 4.1+ but loading extensions in legacy mode.
        if patch == 999 {
            // Godot 4.1+ loading the extension in legacy mode.
            //
            // Instead of panicking, we could *theoretically* fall back to the legacy API at runtime, but then gdext would need to
            // always ship two versions of gdextension_interface.h (+ generated code) and would encourage use of the legacy API.
            panic!(
                "gdext was compiled against a modern Godot version ({static_version}), but loaded in legacy (4.0.x) mode.\n\
                In your .gdextension file, add `compatibility_minimum = 4.1` under the [configuration] section.\n"
            )
        } else {
            // Truly a Godot 4.0 version.
            panic!(
                "gdext was compiled against a newer Godot version ({static_version}),\n\
                but loaded by a legacy Godot binary ({runtime_version}).\n\
                \n\
                You have multiple options:\n\
                1) Run the newer Godot version.\n\
                2) Compile gdext against the older Godot binary (see `custom-godot` feature).\n\
                \n"
            );
        }
    }

    fn runtime_version(&self) -> sys::GDExtensionGodotVersion {
        unsafe {
            let get_proc_address = self.expect("get_proc_address unexpectedly null");
            let get_godot_version = get_proc_address(sys::c_str(b"get_godot_version\0")); //.expect("get_godot_version unexpectedly null");

            let get_godot_version =
                crate::cast_fn_ptr!(get_godot_version as sys::GDExtensionInterfaceGetGodotVersion);

            let mut version = std::mem::MaybeUninit::<sys::GDExtensionGodotVersion>::zeroed();
            get_godot_version(version.as_mut_ptr());
            version.assume_init()
        }
    }

    fn load_interface(&self) -> sys::GDExtensionInterface {
        unsafe { sys::GDExtensionInterface::load(*self) }
    }
}
