/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Legacy 4.0 API

use crate as sys;
use crate::compat::CompatVersion;

pub type InitCompat = *const sys::GDExtensionInterface;

impl CompatVersion for *const sys::GDExtensionInterface {
    fn is_legacy_used_in_modern(&self) -> bool {
        false
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
