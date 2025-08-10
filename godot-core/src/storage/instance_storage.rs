/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::cell::Cell;
use std::ptr;

#[cfg(feature = "experimental-threads")]
use godot_cell::blocking::{InaccessibleGuard, MutGuard, RefGuard};
#[cfg(not(feature = "experimental-threads"))]
use godot_cell::panicking::{InaccessibleGuard, MutGuard, RefGuard};
use godot_ffi as sys;

use crate::godot_error;
use crate::obj::{Base, Gd, GodotClass, Inherits};
use crate::storage::log_pre_drop;

#[derive(Copy, Clone, Debug)]
pub enum Lifecycle {
    // Warning: when reordering/changing enumerators, update match in AtomicLifecycle below
    Alive,
    Destroying,
}

#[cfg_attr(not(feature = "experimental-threads"), allow(dead_code))]
pub struct AtomicLifecycle {
    atomic: std::sync::atomic::AtomicU32,
}

#[cfg_attr(not(feature = "experimental-threads"), allow(dead_code))]
impl AtomicLifecycle {
    pub fn new(value: Lifecycle) -> Self {
        Self {
            atomic: std::sync::atomic::AtomicU32::new(value as u32),
        }
    }

    pub fn get(&self) -> Lifecycle {
        match self.atomic.load(std::sync::atomic::Ordering::Relaxed) {
            0 => Lifecycle::Alive,
            1 => Lifecycle::Destroying,
            other => panic!("invalid lifecycle {other}"),
        }
    }

    pub fn set(&self, lifecycle: Lifecycle) {
        let value = match lifecycle {
            Lifecycle::Alive => 0,
            Lifecycle::Destroying => 1,
        };

        self.atomic
            .store(value, std::sync::atomic::Ordering::Relaxed);
    }
}

/// A storage for an instance binding.
///
/// # Safety
///
/// [`is_bound()`](Storage::is_bound()) must return `true` if any references to the stored user instance
/// exists.
///
/// It must be safe to drop this storage if we have a `&mut` reference to the storage and  
/// [`is_bound()`](Storage::is_bound()) returns `false`.
pub unsafe trait Storage {
    /// The type of instances stored by this storage.
    type Instance: GodotClass;

    /// Constructs a new storage for an instance binding referencing `user_instance`.
    fn construct(
        user_instance: Self::Instance,
        base: Base<<Self::Instance as GodotClass>::Base>,
    ) -> Self;

    /// Returns `true` when there are any outstanding references to this storage's instance.
    fn is_bound(&self) -> bool;

    /// The base object that this storage contains.
    fn base(&self) -> &Base<<Self::Instance as GodotClass>::Base>;

    /// Returns a shared reference to this storage's instance.
    ///
    /// This will ensure Rust's rules surrounding references are upheld. Possibly panicking at runtime if
    /// they are violated.
    fn get(&self) -> RefGuard<'_, Self::Instance>;

    /// Returns a mutable/exclusive reference to this storage's instance.
    ///
    /// This will ensure Rust's rules surrounding references are upheld. Possibly panicking at runtime if
    /// they are violated.
    fn get_mut(&self) -> MutGuard<'_, Self::Instance>;

    /// Returns a guard that allows calling methods on `Gd<Base>` that take `&mut self`.
    ///
    /// This can use the provided `instance` to provide extra safety guarantees such as allowing reentrant
    /// code to create new mutable references.
    fn get_inaccessible<'a: 'b, 'b>(
        &'a self,
        instance: &'b mut Self::Instance,
    ) -> InaccessibleGuard<'b, Self::Instance>;

    /// Returns whether this storage is currently alive or being destroyed.
    ///
    /// This is purely informational and cannot be relied on for safety.
    fn get_lifecycle(&self) -> Lifecycle;

    /// Mark this storage as currently alive or being destroyed.
    ///
    /// This is purely informational and thus is safe to set to whatever value, but it should still be set as
    /// expected.
    fn set_lifecycle(&self, lifecycle: Lifecycle);

    /// Get a `Gd` referencing this storage's instance.
    fn get_gd(&self) -> Gd<Self::Instance>
    where
        Self::Instance: Inherits<<Self::Instance as GodotClass>::Base>,
    {
        self.base().__constructed_gd().cast()
    }

    /// Puts self onto the heap and returns a pointer to this new heap-allocation.
    ///
    /// This will leak memory and so the caller is responsible for manually managing the memory.
    #[must_use]
    fn into_raw(self) -> *mut Self
    where
        Self: Sized,
    {
        Box::into_raw(Box::new(self))
    }

    fn mark_destroyed_by_godot(&self) {
        self.set_lifecycle(Lifecycle::Destroying);

        log_pre_drop(self);
    }

    /// For ref-counted objects, marks as owning a surplus reference.
    ///
    /// Needed when a `Base<T>` hands out extra `Gd<T>` pointers during `init()`, which then requires upgrading the `Base`
    /// weak pointer to a strong one. To compensate, this bool flag will skip the first `inc_ref()` call, which is typically
    /// object construction.
    fn mark_surplus_ref(&self);

    /*#[inline(always)]
    fn destroyed_by_godot(&self) -> bool {
        out!(
            "    is_d;  self={:?}, val={:?}, obj={:?}",
            self as *const _,
            self.get_lifecycle(),
            self.base(),
        );
        matches!(self.get_lifecycle(), Lifecycle::Destroying)
    }*/
}

/// An internal trait for keeping track of reference counts for a storage.
pub(crate) trait StorageRefCounted: Storage {
    fn on_inc_ref(&self);

    fn on_dec_ref(&self);
}

#[cfg(not(feature = "experimental-threads"))]
pub type InstanceStorage<T> = crate::storage::single_threaded::InstanceStorage<T>;

#[cfg(feature = "experimental-threads")]
pub type InstanceStorage<T> = crate::storage::multi_threaded::InstanceStorage<T>;

const fn _assert_implements_storage<T: Storage + StorageRefCounted>() {}

const _INSTANCE_STORAGE_IMPLEMENTS_STORAGE: () =
    _assert_implements_storage::<InstanceStorage<crate::classes::Object>>();

/// Interprets the opaque pointer as pointing to `InstanceStorage<T>`.
///
/// Note: returns reference with unbounded lifetime; intended for local usage
///
/// # Safety
/// `instance_ptr` is assumed to point to a valid instance.
/// The returned reference must be live for the duration of `'u`.
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
            object: {:?}",
            (*raw).base()
        );

        // In Debug mode, crash which may trigger breakpoint.
        // In Release mode, leak player object (Godot philosophy: don't crash if somehow avoidable). Likely leads to follow-up issues.
        if cfg!(debug_assertions) {
            let error = crate::builtin::GString::from(error);
            crate::classes::Os::singleton().crash(&error);
        } else {
            leak_rust_object = true;
            godot_error!("{}", error);
        }
    }

    if !leak_rust_object {
        // SAFETY:
        // `leak_rust_object` is false, meaning that `is_bound()` returned `false`. Because if it were `true`
        // then the process would either be aborted, or we set `leak_rust_object = true`.
        //
        // Therefore, we can safely drop this storage as per the safety contract of `Storage`. Which we know
        // `InstanceStorage<T>` implements because of `_INSTANCE_STORAGE_IMPLEMENTS_STORAGE`.
        let _drop = unsafe { Box::from_raw(raw) };
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// InstanceCache polymorphism, no-op for engine-defined types

pub(crate) trait InstanceCache: Clone {
    fn null() -> Self;
}

impl InstanceCache for () {
    fn null() -> Self {} // returns ()
}

impl InstanceCache for Cell<sys::GDExtensionClassInstancePtr> {
    fn null() -> Self {
        Cell::new(ptr::null_mut())
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
    ptr::null_mut()
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
