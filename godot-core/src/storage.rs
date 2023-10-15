/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::obj::GodotClass;
use crate::out;
use godot_ffi as sys;

#[derive(Copy, Clone, Debug)]
pub enum Lifecycle {
    // Warning: when reordering/changing enumerators, update match in AtomicLifecycle below
    Alive,
    Destroying,
}

#[cfg(not(feature = "experimental-threads"))]
pub(crate) use single_threaded::*;

#[cfg(feature = "experimental-threads")]
pub(crate) use multi_threaded::*;

#[cfg(not(feature = "experimental-threads"))]
mod single_threaded {
    use std::any::type_name;
    use std::cell;

    use crate::obj::{Base, Gd, GodotClass, Inherits};
    use crate::out;

    use super::Lifecycle;

    /// Manages storage and lifecycle of user's extension class instances.
    pub struct InstanceStorage<T: GodotClass> {
        user_instance: cell::RefCell<T>,
        pub(super) base: Base<T::Base>,

        // Declared after `user_instance`, is dropped last
        pub(super) lifecycle: cell::Cell<Lifecycle>,
        godot_ref_count: cell::Cell<u32>,
    }

    /// For all Godot extension classes
    impl<T: GodotClass> InstanceStorage<T> {
        pub fn construct(user_instance: T, base: Base<T::Base>) -> Self {
            out!("    Storage::construct             <{}>", type_name::<T>());

            Self {
                user_instance: cell::RefCell::new(user_instance),
                base,
                lifecycle: cell::Cell::new(Lifecycle::Alive),
                godot_ref_count: cell::Cell::new(1),
            }
        }

        pub(crate) fn on_inc_ref(&self) {
            let refc = self.godot_ref_count.get() + 1;
            self.godot_ref_count.set(refc);

            out!(
                "    Storage::on_inc_ref (rc={})     <{}>", // -- {:?}",
                refc,
                type_name::<T>(),
                //self.user_instance
            );
        }

        pub(crate) fn on_dec_ref(&self) {
            let refc = self.godot_ref_count.get() - 1;
            self.godot_ref_count.set(refc);

            out!(
                "  | Storage::on_dec_ref (rc={})     <{}>", // -- {:?}",
                refc,
                type_name::<T>(),
                //self.user_instance
            );
        }

        pub fn is_bound(&self) -> bool {
            // Needs to borrow mutably, otherwise it succeeds if shared borrows are alive.
            self.user_instance.try_borrow_mut().is_err()
        }

        pub fn get(&self) -> cell::Ref<T> {
            self.user_instance.try_borrow().unwrap_or_else(|_e| {
                panic!(
                    "Gd<T>::bind() failed, already bound; T = {}.\n  \
                     Make sure there is no &mut T live at the time.\n  \
                     This often occurs when calling a GDScript function/signal from Rust, which then calls again Rust code.",
                    type_name::<T>()
                )
            })
        }

        pub fn get_mut(&self) -> cell::RefMut<T> {
            self.user_instance.try_borrow_mut().unwrap_or_else(|_e| {
                panic!(
                    "Gd<T>::bind_mut() failed, already bound; T = {}.\n  \
                     Make sure there is no &T or &mut T live at the time.\n  \
                     This often occurs when calling a GDScript function/signal from Rust, which then calls again Rust code.",
                    type_name::<T>()
                )
            })
        }

        pub fn get_gd(&self) -> Gd<T>
        where
            T: Inherits<<T as GodotClass>::Base>,
        {
            self.base.clone().cast()
        }

        pub(super) fn godot_ref_count(&self) -> u32 {
            self.godot_ref_count.get()
        }
    }
}

#[cfg(feature = "experimental-threads")]
mod multi_threaded {
    use std::sync;
    use std::sync::atomic::{AtomicU32, Ordering};

    use crate::obj::{Base, Gd, GodotClass, Inherits};
    use crate::out;

    use super::Lifecycle;

    pub struct AtomicLifecycle {
        atomic: AtomicU32,
    }

    impl AtomicLifecycle {
        pub fn new(value: Lifecycle) -> Self {
            Self {
                atomic: AtomicU32::new(value as u32),
            }
        }

        pub fn get(&self) -> Lifecycle {
            match self.atomic.load(Ordering::Relaxed) {
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

            self.atomic.store(value, Ordering::Relaxed);
        }
    }

    /// Manages storage and lifecycle of user's extension class instances.
    pub struct InstanceStorage<T: GodotClass> {
        user_instance: sync::RwLock<T>,
        pub(super) base: Base<T::Base>,

        // Declared after `user_instance`, is dropped last
        pub(super) lifecycle: AtomicLifecycle,
        godot_ref_count: AtomicU32,
    }

    /// For all Godot extension classes
    impl<T: GodotClass> InstanceStorage<T> {
        pub fn construct(user_instance: T, base: Base<T::Base>) -> Self {
            out!("    Storage::construct             <{:?}>", base);

            Self {
                user_instance: sync::RwLock::new(user_instance),
                base,
                lifecycle: AtomicLifecycle::new(Lifecycle::Alive),
                godot_ref_count: AtomicU32::new(1),
            }
        }

        pub(crate) fn on_inc_ref(&self) {
            self.godot_ref_count.fetch_add(1, Ordering::Relaxed);
            out!(
                "    Storage::on_inc_ref (rc={})     <{:?}>",
                self.godot_ref_count(),
                self.base,
            );
        }

        pub(crate) fn on_dec_ref(&self) {
            self.godot_ref_count.fetch_sub(1, Ordering::Relaxed);
            out!(
                "  | Storage::on_dec_ref (rc={})     <{:?}>",
                self.godot_ref_count(),
                self.base,
            );
        }

        pub fn is_bound(&self) -> bool {
            // Needs to borrow mutably, otherwise it succeeds if shared borrows are alive.
            self.write_ignoring_poison().is_none()
        }

        pub fn get(&self) -> sync::RwLockReadGuard<T> {
            self.read_ignoring_poison().unwrap_or_else(|| {
                panic!(
                    "Gd<T>::bind() failed, already bound; obj = {}.\n  \
                     Make sure there is no &mut T live at the time.\n  \
                     This often occurs when calling a GDScript function/signal from Rust, which then calls again Rust code.",
                    self.base,
                )
            })
        }

        pub fn get_mut(&self) -> sync::RwLockWriteGuard<T> {
            self.write_ignoring_poison().unwrap_or_else(|| {
                panic!(
                    "Gd<T>::bind_mut() failed, already bound; obj = {}.\n  \
                     Make sure there is no &T or &mut T live at the time.\n  \
                     This often occurs when calling a GDScript function/signal from Rust, which then calls again Rust code.",
                    self.base,
                )
            })
        }

        /// Returns a write guard (even if poisoned), or `None` when the lock is held by another thread.
        /// This might need adjustment if threads should await locks.
        #[must_use]
        fn write_ignoring_poison(&self) -> Option<sync::RwLockWriteGuard<T>> {
            match self.user_instance.try_write() {
                Ok(guard) => Some(guard),
                Err(sync::TryLockError::Poisoned(poison_error)) => Some(poison_error.into_inner()),
                Err(sync::TryLockError::WouldBlock) => None,
            }
        }

        /// Returns a read guard (even if poisoned), or `None` when the lock is held by another writing thread.
        /// This might need adjustment if threads should await locks.
        #[must_use]
        fn read_ignoring_poison(&self) -> Option<sync::RwLockReadGuard<T>> {
            match self.user_instance.try_read() {
                Ok(guard) => Some(guard),
                Err(sync::TryLockError::Poisoned(poison_error)) => Some(poison_error.into_inner()),
                Err(sync::TryLockError::WouldBlock) => None,
            }
        }

        pub fn get_gd(&self) -> Gd<T>
        where
            T: Inherits<<T as GodotClass>::Base>,
        {
            self.base.clone().cast()
        }

        pub(super) fn godot_ref_count(&self) -> u32 {
            self.godot_ref_count.load(Ordering::Relaxed)
        }

        // fn __static_type_check() {
        //     enforce_sync::<InstanceStorage<T>>();
        // }
    }

    // TODO make InstanceStorage<T> Sync
    // This type can be accessed concurrently from multiple threads, so it should be Sync. That implies however that T must be Sync too
    // (and possibly Send, because with `&mut` access, a `T` can be extracted as a value using mem::take() etc.).
    // Which again means that we need to infest half the codebase with T: Sync + Send bounds, *and* make it all conditional on
    // `#[cfg(feature = "experimental-threads")]`. Until the multi-threading design is clarified, we'll thus leave it as is.
    //
    // The following code + __static_type_check() above would make sure that InstanceStorage is Sync.

    // Make sure storage is Sync in multi-threaded case, as it can be concurrently accessed through aliased Gd<T> pointers.
    // fn enforce_sync<T: Sync>() {}
}

impl<T: GodotClass> InstanceStorage<T> {
    pub fn debug_info(&self) -> String {
        // Unlike get_gd(), this doesn't require special trait bounds.

        format!("{:?}", self.base)
    }

    #[must_use]
    pub fn into_raw(self) -> *mut Self {
        Box::into_raw(Box::new(self))
    }

    pub fn mark_destroyed_by_godot(&self) {
        out!(
            "    Storage::mark_destroyed_by_godot", // -- {:?}",
                                                    //self.user_instance
        );
        self.lifecycle.set(Lifecycle::Destroying);
        out!(
            "    mark;  self={:?}, val={:?}, obj={:?}",
            self as *const _,
            self.lifecycle.get(),
            self.base,
        );
    }

    #[inline(always)]
    pub fn destroyed_by_godot(&self) -> bool {
        out!(
            "    is_d;  self={:?}, val={:?}, obj={:?}",
            self as *const _,
            self.lifecycle.get(),
            self.base,
        );
        matches!(self.lifecycle.get(), Lifecycle::Destroying)
    }
}

impl<T: GodotClass> Drop for InstanceStorage<T> {
    fn drop(&mut self) {
        out!(
            "    Storage::drop (rc={})           <{:?}>",
            self.godot_ref_count(),
            self.base,
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

    assert!(
        !(*raw).is_bound(),
        "tried to destroy object while a bind() or bind_mut() call is active\n  \
        object: {}",
        (*raw).debug_info()
    );

    let _drop = Box::from_raw(raw);
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
