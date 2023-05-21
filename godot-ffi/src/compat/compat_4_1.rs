/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Modern 4.1+ API

use crate as sys;

pub type InitCompat = sys::GDExtensionInterfaceGetProcAddress;

impl CompatVersion for sys::GDExtensionInterfaceGetProcAddress {
    fn is_legacy_used_in_modern(&self) -> bool {
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
        let data_ptr = get_proc_address as *const u32; // crowbar it via `as` cast

        // Assumption is that we have at least 8 bytes of memory to safely read from (for both the data and the function case).
        let major = unsafe { data_ptr.read() };
        let minor = unsafe { data_ptr.offset(1).read() };

        return major == 4 && minor == 0;
    }

    fn runtime_version(&self) -> sys::GDExtensionGodotVersion {
        unsafe {
            let get_proc_address = self.expect("get_proc_address unexpectedly null");
            let get_godot_version = get_proc_address(sys::c_str(b"get_godot_version\0")); //.expect("get_godot_version unexpectedly null");

            let get_godot_version = cast_fn_ptr!(get_godot_version as sys::GDExtensionInterfaceGetGodotVersion);

            let mut version = std::mem::MaybeUninit::<sys::GDExtensionGodotVersion>::zeroed();
            get_godot_version(version.as_mut_ptr());
            version.assume_init()
        }
    }

    fn load_interface(&self) -> sys::GDExtensionInterface {
        crate::gen::interface::load_interface()
    }
}
