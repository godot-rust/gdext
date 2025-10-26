/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::obj::{GodotClass, InstanceId};

// This is private; despite `pub` here it is re-exported in `crate::private` module.

/// Object runtime type information, obtained at creation time.
///
/// Stores how a Godot-managed object has been created, for debug info and runtime checks.
/// This is persisted independently of the static type system (e.g. `T` in `Gd<T>`) and can be used to perform sanity checks at runtime.
///
/// See also <https://github.com/godot-rust/gdext/issues/23>.
#[derive(Clone, Debug)]
pub struct ObjectRtti {
    /// Cached instance ID. May point to dead objects.
    instance_id: InstanceId,

    /// Only in Debug mode: dynamic class.
    #[cfg(safeguards_strict)]
    class_name: crate::meta::ClassId,
    //
    // TODO(bromeon): class_id is not always most-derived class; ObjectRtti is sometimes constructed from a base class, via RawGd::from_obj_sys_weak().
    // Examples: after upcast, when receiving Gd<Base> from Godot, etc.
    // Thus, dynamic lookup via Godot get_class() is needed. However, this returns a String, and ClassId is 'static + Copy right now.
}

impl ObjectRtti {
    /// Creates a new instance of `ObjectRtti`.
    #[inline]
    pub fn of<T: GodotClass>(instance_id: InstanceId) -> Self {
        Self {
            instance_id,

            #[cfg(safeguards_strict)]
            class_name: T::class_id(),
        }
    }

    /// Validates that the object's stored type matches or inherits from `T`.
    ///
    /// Used internally by `RawGd::check_rtti()` for type validation in strict mode.
    ///
    /// Only checks the cached type from RTTI construction time.
    /// This may not reflect runtime type changes (which shouldn't happen).
    ///
    /// # Panics (strict safeguards)
    /// If the stored type does not inherit from `T`.
    #[cfg(safeguards_strict)]
    #[inline]
    pub fn check_type<T: GodotClass>(&self) {
        crate::classes::ensure_object_inherits(self.class_name, T::class_id(), self.instance_id);
    }

    #[inline]
    pub fn instance_id(&self) -> InstanceId {
        // Do not add logic or validations here, this is passed in every FFI call.
        self.instance_id
    }
}
