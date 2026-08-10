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

    /// Get a `Gd` referencing the Godot object.
    fn get_gd(&self) -> Gd<Self::Instance>
    where
        Self::Instance: Inherits<<Self::Instance as GodotClass>::Base>,
    {
        self.base().__constructed_gd().cast()
    }

    /// Puts self onto the heap and returns a pointer to this new heap-allocation.
    ///
    /// The allocation is freed once the last claim on it is released; see `StorageClaim`.
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

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Storage claims. The free functions sit at the FFI boundary: they turn Godot's opaque instance pointer into typed access.

/// Keeps the Rust instance storage alive during a Godot -> Rust call.
///
/// User code can drop the last reference to the receiver mid-call (e.g. a signal handler nulling the emitter), or `free()` it. The Godot object
/// then dies while a `GdRef`/`GdMut` guard is still active.
///
/// The storage counts claims: Godot holds one from construction to destruction, each ongoing call holds one. Removing the last claim frees the
/// storage. Guards borrow the claim, so they drop first. A counter rather than a flag, to handle nested calls; atomic under
/// `experimental-threads`.
///
/// Only the storage survives, not the Godot object -- godot-rust cannot prevent its destruction: `Ref::unref()` decides to `memdelete` before
/// the extension's `unreference` callback runs, and `free()` is immediate. Holding a strong reference for the call's duration, like GDScript
/// does, would not help either: it drops before the trampoline returns, fails at refcount 0, and costs ~40% per call. So for the rest of the
/// call, field access works but base operations panic and `Drop` sees a dead base. To survive the call, an object needs a reference that
/// outlives it: `self.to_gd().run_deferred_gd(|_| {})`.
///
/// Entry points which run no user code (`free`, `reference`, `unreference`) use [`as_weak_storage()`] instead.
#[must_use = "dropping the claim immediately releases it, which may free the storage"]
pub struct StorageClaim<T: GodotClass> {
    // Raw pointer rather than reference: the claim owns no borrow (other claims may exist), and freeing needs write access to the whole
    // allocation for drop(&mut). Using a &InstanceStorage shared-ref would be unsound as it would eventually be promoted to &mut.
    storage: *mut InstanceStorage<T>,
}

impl<T: GodotClass> Deref for StorageClaim<T> {
    type Target = InstanceStorage<T>;

    fn deref(&self) -> &Self::Target {
        // SAFETY: the claim keeps the storage alive.
        unsafe { &*self.storage }
    }
}

impl<T: GodotClass> Drop for StorageClaim<T> {
    fn drop(&mut self) {
        if self.dec_claims() {
            // SAFETY: this was the last claim, and the guards borrowing it are already dropped.
            unsafe { deallocate_storage(self.storage) };
        }
    }
}

/// Deallocates the storage, once no claim is left.
///
/// # Safety
/// The last claim on `storage` must have been released, and no reference into it may be live.
unsafe fn deallocate_storage<T: GodotClass>(storage: *mut InstanceStorage<T>) {
    // SAFETY: delegated to caller.
    let storage_ref = unsafe { &*storage };

    // A guard still points into the storage -> leak instead of freeing, so its references stay valid.
    // Only reachable for guards that hold no "claim", i.e. `Gd::bind()` outside a Godot->Rust call.
    // Guards within such calls borrow the claim and thus drop before it.
    if storage_ref.is_bound() {
        report_destruction_while_bound(storage_ref); // May crash process, depending on safeguard level.
        return;
    }

    // SAFETY: no claim is left, so no call is ongoing and no other reference into the storage exists -- guards borrow the claim, and
    // `is_bound()` ruled out stray ones. Dropping is thus allowed by the safety contract of `Storage`, which `InstanceStorage<T>`
    // implements (see `_INSTANCE_STORAGE_IMPLEMENTS_STORAGE`). The allocation stems from `Storage::into_raw()`.
    let _drop = unsafe { Box::from_raw(storage) };
}

/// Interprets the opaque pointer as pointing to `InstanceStorage<T>`. Sole place deriving storage pointers from Godot's instance pointer.
fn as_storage_ptr<T: GodotClass>(
    instance_ptr: sys::GDExtensionClassInstancePtr,
) -> *mut InstanceStorage<T> {
    instance_ptr as *mut InstanceStorage<T>
}

/// Interprets the opaque pointer as pointing to `InstanceStorage<T>`, keeping the storage alive until the returned claim drops.
///
/// # Safety
/// `instance_ptr` is assumed to point to a valid instance.
///
/// The claim keeps the storage (Rust object) alive, not the Godot object -- the latter may die mid-call.
pub unsafe fn as_storage<T: GodotClass>(
    instance_ptr: sys::GDExtensionClassInstancePtr,
) -> StorageClaim<T> {
    let storage = as_storage_ptr::<T>(instance_ptr);

    // SAFETY: delegated to caller.
    unsafe { (*storage).inc_claims() };

    StorageClaim { storage }
}

/// Like [`as_storage()`], but *weak*: the returned reference does not keep the storage alive. Only for entry points that run no user code.
///
/// # Safety
/// Same as [`as_storage()`]. Additionally, the storage must not be destroyed while the returned reference is live.
pub unsafe fn as_weak_storage<'u, T: GodotClass>(
    instance_ptr: sys::GDExtensionClassInstancePtr,
) -> &'u InstanceStorage<T> {
    // SAFETY: delegated to caller.
    unsafe { &*as_storage_ptr::<T>(instance_ptr) }
}

/// Releases the claim Godot took when the storage was constructed; see [`StorageClaim`].
///
/// Deallocates the storage, unless a Godot -> Rust call still holds a claim -- then the last of those calls deallocates it.
///
/// # Safety
/// `instance_ptr` is assumed to point to a valid instance. This function must only be invoked once for a pointer.
pub unsafe fn release_godot_claim<T: GodotClass>(instance_ptr: sys::GDExtensionClassInstancePtr) {
    let storage = as_storage_ptr::<T>(instance_ptr);

    // SAFETY: Godot's claim exists since construction, and the caller guarantees to release it only once.
    let was_last_claim = unsafe { (*storage).dec_claims() };

    if was_last_claim {
        // SAFETY: no claim is left; each entry point running user code holds one for its duration.
        unsafe { deallocate_storage(storage) };
    }
}

/// Reports that the storage is destroyed while the Rust object is still borrowed. May crash the process, depending on safeguard level.
fn report_destruction_while_bound<T: GodotClass>(storage: &InstanceStorage<T>) {
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
        "Destroyed Godot object during active bind() or bind_mut() guard; object: {:?}.\n  \
        This is a bug in your code that may cause UB and logic errors. Make sure that objects are not\n  \
        destroyed while you still hold a Rust reference to them, or use Gd::free() which is safe.",
        storage.base()
    );

    // In strict+balanced level, crash which may trigger breakpoint.
    // In disengaged level, leak Rust object (Godot philosophy: don't crash if somehow avoidable). Likely leads to follow-up issues.
    if cfg!(safeguards_balanced) {
        let error = crate::builtin::GString::from(&error);
        crate::classes::Os::singleton().crash(&error);
    } else {
        godot_error!("{}", error);
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
