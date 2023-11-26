/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::ops::{Deref, DerefMut};

use crate::obj::{Base, Gd, GodotClass, Inherits};
use crate::{godot_error, out};
use godot_ffi as sys;

#[derive(Copy, Clone, Debug)]
pub enum Lifecycle {
    // Warning: when reordering/changing enumerators, update match in AtomicLifecycle below
    Alive,
    Destroying,
}

#[cfg_attr(feature = "experimental-threads", allow(dead_code))]
mod single_threaded;

#[cfg_attr(not(feature = "experimental-threads"), allow(dead_code))]
mod multi_threaded;

pub trait Storage {
    type Instance: GodotClass;
    type RefGuard<'a>: Deref<Target = Self::Instance>
    where
        Self: 'a;
    type MutGuard<'a>: Deref<Target = Self::Instance> + DerefMut
    where
        Self: 'a;

    fn construct(
        user_instance: Self::Instance,
        base: Base<<Self::Instance as GodotClass>::Base>,
    ) -> Self;

    fn is_bound(&self) -> bool;

    fn base(&self) -> &Base<<Self::Instance as GodotClass>::Base>;

    fn get(&self) -> Self::RefGuard<'_>;

    fn get_mut(&self) -> Self::MutGuard<'_>;

    fn get_lifecycle(&self) -> Lifecycle;

    fn set_lifecycle(&self, lifecycle: Lifecycle);

    fn get_gd(&self) -> Gd<Self::Instance>
    where
        Self::Instance: Inherits<<Self::Instance as GodotClass>::Base>,
    {
        self.base().to_gd().cast()
    }

    fn debug_info(&self) -> String {
        // Unlike get_gd(), this doesn't require special trait bounds.

        format!("{:?}", self.base())
    }

    #[must_use]
    fn into_raw(self) -> *mut Self
    where
        Self: Sized,
    {
        Box::into_raw(Box::new(self))
    }

    fn mark_destroyed_by_godot(&self) {
        out!(
            "    Storage::mark_destroyed_by_godot", // -- {:?}",
                                                    //self.user_instance
        );
        self.set_lifecycle(Lifecycle::Destroying);
        out!(
            "    mark;  self={:?}, val={:?}, obj={:?}",
            self as *const _,
            self.get_lifecycle(),
            self.base(),
        );
    }

    #[inline(always)]
    fn destroyed_by_godot(&self) -> bool {
        out!(
            "    is_d;  self={:?}, val={:?}, obj={:?}",
            self as *const _,
            self.get_lifecycle(),
            self.base(),
        );
        matches!(self.get_lifecycle(), Lifecycle::Destroying)
    }
}

pub(crate) trait StorageRefCounted: Storage {
    fn godot_ref_count(&self) -> u32;

    fn on_inc_ref(&self);

    fn on_dec_ref(&self);
}

#[cfg(not(feature = "experimental-threads"))]
pub type InstanceStorage<T> = single_threaded::InstanceStorage<T>;
#[cfg(feature = "experimental-threads")]
pub type InstanceStorage<T> = multi_threaded::InstanceStorage<T>;

pub type RefGuard<'a, T> = <InstanceStorage<T> as Storage>::RefGuard<'a>;
pub type MutGuard<'a, T> = <InstanceStorage<T> as Storage>::MutGuard<'a>;

impl<T: GodotClass> Drop for InstanceStorage<T> {
    fn drop(&mut self) {
        out!(
            "    Storage::drop (rc={})           <{:?}>",
            self.godot_ref_count(),
            self.base(),
        );
        //let _ = mem::take(&mut self.user_instance);
        //out!("    Storage::drop end              <{:?}>", self.base);
    }
}

/// Interprets the opaque pointer as pointing to `InstanceStorage<T>`.
///
/// Note: returns reference with unbounded lifetime; intended for local usage
///
/// # Safety
/// `instance_ptr` is assumed to point to a valid instance.
// Note: unbounded ref AND &mut out of thin air is not very beautiful, but it's  -- consider using with_storage(ptr, closure) and drop_storage(ptr)
pub unsafe fn as_storage<'u, T: GodotClass>(
    instance_ptr: sys::GDExtensionClassInstancePtr,
) -> &'u InstanceStorage<T> {
    &*(instance_ptr as *mut InstanceStorage<T>)
}

/// # Safety
/// `instance_ptr` is assumed to point to a valid instance. This function must only be invoked once for a pointer.
pub unsafe fn destroy_storage<T: GodotClass>(instance_ptr: sys::GDExtensionClassInstancePtr) {
    let raw = instance_ptr as *mut InstanceStorage<T>;

    // We cannot panic here, since this code is invoked from a C callback. Panicking would mean unwinding into C code, which is UB.
    // We have the following options:
    // 1. Print an error as a best-effort, knowing that UB is likely to occur whenever the user will access &T or &mut T. (Technically, the
    //    mere existence of these references is UB since the T is dead.)
    // 2. Abort the process. This is the safest option, but a very drastic measure, and not what gdext does elsewhere.
    //    We can use Godot's OS.crash() API here.
    // 3. Change everything to "C-unwind" API. Would make the FFI unwinding safe, but still not clear if Godot would handle it appropriately.
    //    Even if yes, it's likely the same behavior as OS.crash().
    // 4. Prevent destruction of the Rust part (InstanceStorage). This would solve the immediate problem of &T and &mut T becoming invalid,
    //    but it would leave a zombie object behind, where all base operations and Godot interactions suddenly fail, which likely creates
    //    its own set of edge cases. It would _also_ make the problem less observable, since the user can keep interacting with the Rust
    //    object and slowly accumulate memory leaks.
    //    - Letting Gd<T> and InstanceStorage<T> know about this specific object state and panicking in the next Rust call might be an option,
    //      but we still can't control direct access to the T.
    //
    // For now we choose option 2 in Debug mode, and 4 in Release.
    let mut leak_rust_object = false;
    if (*raw).is_bound() {
        let error = format!(
            "Destroyed an object from Godot side, while a bind() or bind_mut() call was active.\n  \
            This is a bug in your code that may cause UB and logic errors. Make sure that objects are not\n  \
            destroyed while you still hold a Rust reference to them, or use Gd::free() which is safe.\n  \
            object: {}",
            (*raw).debug_info()
        );

        // In Debug mode, crash which may trigger breakpoint.
        // In Release mode, leak player object (Godot philosophy: don't crash if somehow avoidable). Likely leads to follow-up issues.
        if cfg!(debug_assertions) {
            crate::engine::Os::singleton().crash(error.into());
        } else {
            leak_rust_object = true;
            godot_error!("{}", error);
        }
    }

    if !leak_rust_object {
        let _drop = Box::from_raw(raw);
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Callbacks

pub fn nop_instance_callbacks() -> sys::GDExtensionInstanceBindingCallbacks {
    // These could also be null pointers, if they are definitely not invoked (e.g. create_callback only passed to object_get_instance_binding(),
    // when there is already a binding). Current "empty but not null" impl corresponds to godot-cpp (wrapped.hpp).
    sys::GDExtensionInstanceBindingCallbacks {
        create_callback: Some(create_callback),
        free_callback: Some(free_callback),
        reference_callback: Some(reference_callback),
    }
}

extern "C" fn create_callback(
    _p_token: *mut std::os::raw::c_void,
    _p_instance: *mut std::os::raw::c_void,
) -> *mut std::os::raw::c_void {
    // There is no "instance binding" for Godot types like Node3D -- this would be the user-defined Rust class
    std::ptr::null_mut()
}

extern "C" fn free_callback(
    _p_token: *mut std::os::raw::c_void,
    _p_instance: *mut std::os::raw::c_void,
    _p_binding: *mut std::os::raw::c_void,
) {
}

extern "C" fn reference_callback(
    _p_token: *mut std::os::raw::c_void,
    _p_binding: *mut std::os::raw::c_void,
    _p_reference: sys::GDExtensionBool,
) -> sys::GDExtensionBool {
    true as u8
}
