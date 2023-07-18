/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::obj::GodotClass;
use crate::out;
use godot_ffi as sys;

use std::any::type_name;

#[derive(Copy, Clone, Debug)]
pub enum Lifecycle {
    Alive,
    Destroying,
    Dead, // reading this would typically already be too late, only best-effort in case of UB
}

#[cfg(not(feature = "threads"))]
pub use single_thread::*;

#[cfg(feature = "threads")]
pub use multi_thread::*;

#[cfg(not(feature = "threads"))]
mod single_thread {
    use std::any::type_name;
    use std::cell;

    use crate::obj::GodotClass;
    use crate::out;

    use super::Lifecycle;

    /// Manages storage and lifecycle of user's extension class instances.
    pub struct InstanceStorage<T: GodotClass> {
        user_instance: cell::RefCell<T>,

        // Declared after `user_instance`, is dropped last
        pub lifecycle: cell::Cell<Lifecycle>,
        godot_ref_count: cell::Cell<u32>,
    }

    /// For all Godot extension classes
    impl<T: GodotClass> InstanceStorage<T> {
        pub fn construct(user_instance: T) -> Self {
            out!("    Storage::construct             <{}>", type_name::<T>());

            Self {
                user_instance: cell::RefCell::new(user_instance),
                lifecycle: cell::Cell::new(Lifecycle::Alive),
                godot_ref_count: cell::Cell::new(1),
            }
        }

        pub(crate) fn on_inc_ref(&self) {
            self.godot_ref_count.set(self.godot_ref_count() + 1);
            out!(
                "    Storage::on_inc_ref (rc={})     <{}>", // -- {:?}",
                self.godot_ref_count(),
                type_name::<T>(),
                //self.user_instance
            );
        }

        pub(crate) fn on_dec_ref(&self) {
            self.godot_ref_count.set(self.godot_ref_count() - 1);
            out!(
                "  | Storage::on_dec_ref (rc={})     <{}>", // -- {:?}",
                self.godot_ref_count(),
                type_name::<T>(),
                //self.user_instance
            );
        }

        /* pub fn destroy(&mut self) {
            assert!(
                self.user_instance.is_some(),
                "Cannot destroy user instance which is not yet initialized"
            );
            assert!(
                !self.destroyed,
                "Cannot destroy user instance multiple times"
            );
            self.user_instance = None; // drops T
                                       // TODO drop entire Storage
        }*/

        pub fn set_lifecycle(&self, lifecycle: Lifecycle) {
            self.lifecycle.set(lifecycle)
        }

        pub fn lifecycle(&self) -> Lifecycle {
            self.lifecycle.get()
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

        pub(super) fn godot_ref_count(&self) -> u32 {
            self.godot_ref_count.get()
        }
    }
}

#[cfg(feature = "threads")]
mod multi_thread {
    use std::any::type_name;
    use std::sync;
    use std::sync::atomic::{AtomicU32, Ordering};

    use crate::obj::GodotClass;
    use crate::out;

    use super::Lifecycle;

    /// Manages storage and lifecycle of user's extension class instances.
    pub struct InstanceStorage<T: GodotClass> {
        user_instance: sync::RwLock<T>,

        // Declared after `user_instance`, is dropped last
        pub lifecycle: Lifecycle,
        godot_ref_count: AtomicU32,
    }

    /// For all Godot extension classes
    impl<T: GodotClass> InstanceStorage<T> {
        pub fn construct(user_instance: T) -> Self {
            out!("    Storage::construct             <{}>", type_name::<T>());

            Self {
                user_instance: sync::RwLock::new(user_instance),
                lifecycle: Lifecycle::Alive,
                godot_ref_count: AtomicU32::new(1),
            }
        }

        pub(crate) fn on_inc_ref(&mut self) {
            self.godot_ref_count.fetch_add(1, Ordering::Relaxed);
            out!(
                "    Storage::on_inc_ref (rc={})     <{}>", // -- {:?}",
                self.godot_ref_count(),
                type_name::<T>(),
                //self.user_instance
            );
        }

        pub(crate) fn on_dec_ref(&mut self) {
            self.godot_ref_count.fetch_sub(1, Ordering::Relaxed);
            out!(
                "  | Storage::on_dec_ref (rc={})     <{}>", // -- {:?}",
                self.godot_ref_count(),
                type_name::<T>(),
                //self.user_instance
            );
        }

        /* pub fn destroy(&mut self) {
            assert!(
                self.user_instance.is_some(),
                "Cannot destroy user instance which is not yet initialized"
            );
            assert!(
                !self.destroyed,
                "Cannot destroy user instance multiple times"
            );
            self.user_instance = None; // drops T
                                       // TODO drop entire Storage
        }*/

        pub fn get(&self) -> sync::RwLockReadGuard<T> {
            self.user_instance.read().unwrap_or_else(|_e| {
                panic!(
                    "Gd<T>::bind() failed, already bound; T = {}.\n  \
                     Make sure there is no &mut T live at the time.\n  \
                     This often occurs when calling a GDScript function/signal from Rust, which then calls again Rust code.",
                    type_name::<T>()
                )
            })
        }

        pub fn get_mut(&mut self) -> sync::RwLockWriteGuard<T> {
            self.user_instance.write().unwrap_or_else(|_e| {
                panic!(
                    "Gd<T>::bind_mut() failed, already bound; T = {}.\n  \
                     Make sure there is no &T or &mut T live at the time.\n  \
                     This often occurs when calling a GDScript function/signal from Rust, which then calls again Rust code.",
                    type_name::<T>()
                )
            })
        }

        pub(super) fn godot_ref_count(&self) -> u32 {
            self.godot_ref_count.load(Ordering::Relaxed)
        }
    }
}

impl<T: GodotClass> InstanceStorage<T> {
    #[must_use]
    pub fn into_raw(self) -> *mut Self {
        Box::into_raw(Box::new(self))
    }

    pub fn mark_destroyed_by_godot(&self) {
        out!(
            "    Storage::mark_destroyed_by_godot", // -- {:?}",
                                                    //self.user_instance
        );
        self.set_lifecycle(Lifecycle::Destroying);
        out!(
            "    mark;  self={:?}, val={:?}",
            self as *const _,
            self.lifecycle()
        );
    }

    #[inline(always)]
    pub fn destroyed_by_godot(&self) -> bool {
        out!(
            "    is_d;  self={:?}, val={:?}",
            self as *const _,
            self.lifecycle()
        );
        matches!(self.lifecycle(), Lifecycle::Destroying | Lifecycle::Dead)
    }
}

impl<T: GodotClass> Drop for InstanceStorage<T> {
    fn drop(&mut self) {
        out!(
            "    Storage::drop (rc={})           <{}>", // -- {:?}",
            self.godot_ref_count(),
            type_name::<T>(),
            //self.user_instance
        );
        //let _ = mem::take(&mut self.user_instance);
        out!(
            "    Storage::drop end              <{}>", //  -- {:?}",
            type_name::<T>(),
            //self.user_instance
        );
    }
}

/// Interprets the opaque pointer as pointing to `InstanceStorage<T>`.
///
/// Note: returns reference with unbounded lifetime; intended for local usage
///
/// # Safety
/// `instance_ptr` is assumed to point to a valid instance.
// FIXME unbounded ref AND &mut out of thin air is a huge hazard -- consider using with_storage(ptr, closure) and drop_storage(ptr)
pub unsafe fn as_storage<'u, T: GodotClass>(
    instance_ptr: sys::GDExtensionClassInstancePtr,
) -> &'u InstanceStorage<T> {
    &*(instance_ptr as *const InstanceStorage<T>)
}

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
