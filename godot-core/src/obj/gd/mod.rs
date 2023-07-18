/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot_ffi as sys;
use sys::static_assert_eq_size;

use std::ptr;

mod base;
#[allow(clippy::module_inception)]
mod gd;
mod raw_gd;
mod singleton;

pub use base::Base;
pub use gd::Gd;
pub use raw_gd::RawGd;
pub use singleton::Singleton;

use super::{GodotClass, InstanceId};

// Size equality check, while we dont currently use `OpaqueObject` we should still check that the size
// inferred from the `JSON` file matches the pointer width.
static_assert_eq_size!(
    sys::GDExtensionObjectPtr,
    sys::types::OpaqueObject,
    "Godot FFI: pointer type `Object*` should have size advertised in JSON extension file"
);

/// Runs `init_fn` on the address of a pointer (initialized to null), then returns that pointer, possibly still null.
///
/// # Safety
/// `init_fn` must be a function that correctly handles a _type pointer_ pointing to an _object pointer_.
#[doc(hidden)]
pub unsafe fn raw_object_init(
    init_fn: impl FnOnce(sys::GDExtensionUninitializedTypePtr),
) -> sys::GDExtensionObjectPtr {
    // return_ptr has type GDExtensionTypePtr = GDExtensionObjectPtr* = OpaqueObject* = Object**
    // (in other words, the type-ptr contains the _address_ of an object-ptr).
    let mut object_ptr: sys::GDExtensionObjectPtr = ptr::null_mut();
    let return_ptr: *mut sys::GDExtensionObjectPtr = ptr::addr_of_mut!(object_ptr);

    init_fn(return_ptr as sys::GDExtensionUninitializedTypePtr);

    // We don't need to know if Object** is null, but if Object* is null; return_ptr has the address of a local (never null).
    object_ptr
}

pub trait GodotObjectPtr {
    type Class: GodotClass;

    /// Get the underlying raw object from this smart pointer.
    fn raw(&self) -> &RawGd<Self::Class>;

    /// Returns the instance ID of this object, or `None` if no instance ID is cached.
    ///
    /// This function does not check that the returned instance ID points to a valid instance!
    /// Unless performance is a problem, use [`instance_id_or_none`].
    fn instance_id_or_none_unchecked(&self) -> Option<InstanceId> {
        self.raw().instance_id_or_none_unchecked()
    }

    /// Returns the instance ID of this object, or `None` if the object is dead.
    fn instance_id_or_none(&self) -> Option<InstanceId> {
        self.raw().instance_id_or_none()
    }

    /// ⚠️ Returns the instance ID of this object (panics when dead).
    ///
    /// # Panics
    /// If this object is no longer alive (registered in Godot's object database).
    fn instance_id(&self) -> InstanceId {
        self.instance_id_or_none().unwrap_or_else(|| {
            panic!(
                "failed to call instance_id() on destroyed object; \
                use instance_id_or_none() or keep your objects alive"
            )
        })
    }

    /// Checks if this smart pointer points to a live object (read description!).
    ///
    /// Using this method is often indicative of bad design -- you should dispose of your pointers once an object is
    /// destroyed. However, this method exists because GDScript offers it and there may be **rare** use cases.
    ///
    /// Do not use this method to check if you can safely access an object. Accessing dead objects is generally safe
    /// and will panic in a defined manner. Encountering such panics is almost always a bug you should fix, and not a
    /// runtime condition to check against.
    fn is_instance_valid(&self) -> bool {
        self.instance_id_or_none().is_some()
    }
}
