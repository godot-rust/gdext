/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::cell::Cell;
use std::ops::{Deref, DerefMut};
use std::ptr;

#[cfg(feature = "experimental-threads")]
use godot_cell::blocking::{InaccessibleGuard, MutGuard, RefGuard};
#[cfg(not(feature = "experimental-threads"))]
use godot_cell::panicking::{InaccessibleGuard, MutGuard, RefGuard};
use godot_ffi as sys;

use crate::godot_error;
use crate::obj::{Base, Gd, GodotClass, Inherits, Singleton};
use crate::storage::log_pre_drop;

sys::atomic_enum! {
    #[derive(Copy, Clone, Debug)]
    pub enum Lifecycle {
        Alive = 0,
        Destroying = 1,
    }
}

/// Type-safe atomic wrapper for [`Lifecycle`].
#[cfg_attr(not(feature = "experimental-threads"), allow(dead_code))]
pub type AtomicLifecycle = sys::AtomicEnum<Lifecycle>;

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

    /// Adds a share to the storage's lifetime count; see [`RetainedStorage`].
    ///
    /// Must be paired with exactly one [`release()`][Self::release].
    fn retain(&self);

    /// Removes a share added by [`retain()`][Self::retain]; returns `true` if it was the last one, i.e. the storage must now be freed.
    ///
    /// Godot owns the share created together with the storage, and releases it through [`destroy_storage()`].
    fn release(&self) -> bool;

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

/// Wrapper to handle multiple receivers type, without exposing the Storage itself.
#[doc(hidden)]
pub struct VirtualMethodReceiver<'a, T: GodotClass> {
    inner: VirtualMethodReceiverInner<'a, T>,
}

enum VirtualMethodReceiverInner<'a, T: GodotClass> {
    /// &self.
    Ref(RefGuard<'a, T>),
    /// &mut self.
    Mut(MutGuard<'a, T>),
    /// this: Gd<Self>.
    GdSelf(Gd<T>),
    /// Implementation detail – required to swap the values.
    Uninit,
}

impl<'a, T: GodotClass> VirtualMethodReceiver<'a, T> {
    pub fn recv_gd(mut self) -> Gd<T> {
        match std::mem::replace(&mut self.inner, VirtualMethodReceiverInner::Uninit) {
            VirtualMethodReceiverInner::GdSelf(instance) => instance,
            _ => panic!("Tried to use Gd<Self> receiver for method which doesn't accept it."),
        }
    }

    pub fn recv_self(mut self) -> impl Deref<Target = T> + use<'a, T> {
        match std::mem::replace(&mut self.inner, VirtualMethodReceiverInner::Uninit) {
            VirtualMethodReceiverInner::Ref(instance) => instance,
            _ => panic!("Tried to use &self receiver for method which doesn't accept it."),
        }
    }

    pub fn recv_self_mut(mut self) -> impl DerefMut<Target = T> + use<'a, T> {
        match std::mem::replace(&mut self.inner, VirtualMethodReceiverInner::Uninit) {
            VirtualMethodReceiverInner::Mut(instance) => instance,
            _ => panic!("Tried to use &mut self receiver for method which doesn't accept it."),
        }
    }
}

// Marker structs.
// Used to extract proper type from storage and pass it to public API while defined as an associated item on the trait (`T::Recv::instance(storage)`).

#[doc(hidden)]
pub enum RecvRef {}
#[doc(hidden)]
pub enum RecvMut {}
#[doc(hidden)]
pub enum RecvGdSelf {}

#[doc(hidden)]
pub trait IntoVirtualMethodReceiver<T: GodotClass> {
    #[doc(hidden)]
    fn instance<'a, 'b: 'a>(storage: &'b InstanceStorage<T>) -> VirtualMethodReceiver<'a, T>;
}

impl<T: GodotClass> IntoVirtualMethodReceiver<T> for RecvRef {
    fn instance<'a, 'b: 'a>(storage: &'b InstanceStorage<T>) -> VirtualMethodReceiver<'a, T> {
        VirtualMethodReceiver {
            inner: VirtualMethodReceiverInner::Ref(storage.get()),
        }
    }
}

impl<T: GodotClass> IntoVirtualMethodReceiver<T> for RecvMut {
    fn instance<'a, 'b: 'a>(storage: &'b InstanceStorage<T>) -> VirtualMethodReceiver<'a, T> {
        VirtualMethodReceiver {
            inner: VirtualMethodReceiverInner::Mut(storage.get_mut()),
        }
    }
}

impl<T> IntoVirtualMethodReceiver<T> for RecvGdSelf
where
    T: Inherits<<T as GodotClass>::Base>,
{
    fn instance<'a, 'b: 'a>(storage: &'b InstanceStorage<T>) -> VirtualMethodReceiver<'a, T> {
        VirtualMethodReceiver {
            inner: VirtualMethodReceiverInner::GdSelf(storage.get_gd()),
        }
    }
}

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
    unsafe { &*(instance_ptr as *mut InstanceStorage<T>) }
}

/// Storage access for a Godot -> Rust call into user code, keeping the storage alive for the duration of the call.
///
/// # Problem
/// A callback from Godot receives only the instance pointer, so the call frame holds no share of the object's reference count. If user code
/// drops the last reference mid-call -- e.g. a signal handler nulling the emitter -- object and storage are destroyed while an instance
/// guard is still active. Manually managed classes reach the same state through `free()`.
///
/// # Mechanism
/// The storage counts shares of its own lifetime: Godot holds one from construction, each call holds one for as long as this handle lives,
/// and whoever releases the last share frees the storage. Guards borrow this handle, so the compiler enforces that they are released first.
/// Counting (rather than a flag) is what makes reentrant calls work.
///
/// One counter, rather than a flag plus a counter, so exactly one side observes itself as last -- two words could miss each other under
/// `experimental-threads` and leak the storage.
///
/// # Trade-off
/// This keeps the *storage* alive, not the object: the object dies as soon as the last reference goes away, so for the rest of the call,
/// `&self`/`&mut self` field access stays valid while base operations panic (`ensure_object_alive()`, at default safeguard levels). A `Drop`
/// impl of the user instance likewise sees a dead base, unlike in ordinary destruction.
///
/// The alternative is to hold a `Gd` reference per call, keeping the object itself alive and matching GDScript's semantics. It was measured
/// at +42% per call into a ref-counted class, and being a reference, it does nothing for manually managed classes -- which need this counter
/// regardless.
///
/// Entry points which run no user code (`free`, `reference`, `unreference`) use [`as_storage()`] instead.
pub struct RetainedStorage<'u, T: GodotClass> {
    storage: &'u InstanceStorage<T>,
}

impl<T: GodotClass> Deref for RetainedStorage<'_, T> {
    type Target = InstanceStorage<T>;

    fn deref(&self) -> &Self::Target {
        self.storage
    }
}

impl<T: GodotClass> Drop for RetainedStorage<'_, T> {
    fn drop(&mut self) {
        // SAFETY: this handle owns the share added by as_retained_storage(), and guards derived from it are gone by construction.
        unsafe { release_storage(self.storage) };
    }
}

/// Like [`as_storage()`], but keeps the storage alive for the returned handle's lifetime; see [`RetainedStorage`].
///
/// # Safety
/// Same as [`as_storage()`].
pub unsafe fn as_retained_storage<'u, T: GodotClass>(
    instance_ptr: sys::GDExtensionClassInstancePtr,
) -> RetainedStorage<'u, T> {
    // SAFETY: delegated to caller.
    let storage = unsafe { as_storage::<T>(instance_ptr) };
    storage.retain();

    RetainedStorage { storage }
}

/// # Safety
/// `instance_ptr` is assumed to point to a valid instance. This function must only be invoked once for a pointer.
pub unsafe fn destroy_storage<T: GodotClass>(instance_ptr: sys::GDExtensionClassInstancePtr) {
    // SAFETY: valid pointer; shared reference, since an in-flight call holds one as well.
    let storage = unsafe { as_storage::<T>(instance_ptr) };

    // SAFETY: releases Godot's share, which the caller guarantees to release only once.
    unsafe { release_storage(storage) };
}

/// Releases one share of `storage`'s lifetime and frees it if that was the last one; see [`RetainedStorage`].
///
/// # Safety
/// The caller must own a share added by [`Storage::retain()`] or by construction, and must not release it more than once. No references into
/// the storage other than `storage` itself may exist.
unsafe fn release_storage<T: GodotClass>(storage: &InstanceStorage<T>) {
    if !storage.release() {
        return;
    }

    // A guard smuggled out of a call's borrow (e.g. leaked) still points into the storage; report and leak instead of freeing.
    if leak_if_bound(storage) {
        return;
    }

    let raw = ptr::from_ref(storage).cast_mut();

    // SAFETY: the last share is gone, so no call is in flight and no other reference into the storage exists; `leak_if_bound()` ruled out
    // stray guards. Dropping is therefore allowed by the safety contract of `Storage`, which `InstanceStorage<T>` implements (see
    // `_INSTANCE_STORAGE_IMPLEMENTS_STORAGE`).
    let _drop = unsafe { Box::from_raw(raw) };
}

/// Reports a storage destruction while the user instance is still borrowed; returns `true` if the storage must be leaked instead of freed.
fn leak_if_bound<T: GodotClass>(storage: &InstanceStorage<T>) -> bool {
    if !storage.is_bound() {
        return false;
    }

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
    // For now we choose option 2 in strict+balanced levels, and 4 in disengaged level.
    let error = format!(
        "Destroyed an object from Godot side, while a bind() or bind_mut() call was active.\n  \
        This is a bug in your code that may cause UB and logic errors. Make sure that objects are not\n  \
        destroyed while you still hold a Rust reference to them, or use Gd::free() which is safe.\n  \
        object: {:?}",
        storage.base()
    );

    // In strict+balanced level, crash which may trigger breakpoint.
    // In disengaged level, leak player object (Godot philosophy: don't crash if somehow avoidable). Likely leads to follow-up issues.
    if cfg!(safeguards_balanced) {
        let error = crate::builtin::GString::from(&error);
        crate::classes::Os::singleton().crash(&error);
        false
    } else {
        godot_error!("{}", error);
        true
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
