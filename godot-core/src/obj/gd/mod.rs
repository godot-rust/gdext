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

pub use base::Base;
pub use gd::Gd;
pub use raw_gd::RawGd;

// Size equality check (should additionally be covered by mem::transmute())
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
