/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

#[cfg(feature = "experimental-threads")]
use godot_cell::blocking::{GdCell, InaccessibleGuard, MutGuard, RefGuard};
#[cfg(not(feature = "experimental-threads"))]
use godot_cell::panicking::{GdCell, InaccessibleGuard, MutGuard, RefGuard};

use crate::obj::{Base, GodotClass};
use crate::storage::{AtomicLifecycle, DebugBorrowTracker, Lifecycle, Storage, StorageRefCounted};

pub struct InstanceStorage<T: GodotClass> {
    user_instance: GdCell<T>,
    pub(super) base: Base<T::Base>,

    // Declared after `user_instance`, is dropped last
    pub(super) lifecycle: AtomicLifecycle,

    // No-op in Release mode.
    borrow_tracker: DebugBorrowTracker,
}

// SAFETY:
// The only way to get a reference to the user instance is by going through the `GdCell` in `user_instance`.
// If this `GdCell` has returned any references, then `self.user_instance.as_ref().is_currently_bound()` will
// return true. So `is_bound` will return true when a reference to the user instance exists.
//
// If `is_bound` is false, then there are no references to the user instance in this storage. And if a `&mut`
// reference to the storage exists then no other references to data in this storage can exist. So we can
// safely drop it.
unsafe impl<T: GodotClass> Storage for InstanceStorage<T> {
    type Instance = T;

    fn construct(
        user_instance: Self::Instance,
        base: Base<<Self::Instance as GodotClass>::Base>,
    ) -> Self {
        super::log_construct::<T>(&base);

        Self {
            user_instance: GdCell::new(user_instance),
            base,
            lifecycle: AtomicLifecycle::new(Lifecycle::Alive),
            borrow_tracker: DebugBorrowTracker::new(),
        }
    }

    fn is_bound(&self) -> bool {
        self.user_instance.is_currently_bound()
    }

    fn base(&self) -> &Base<<Self::Instance as GodotClass>::Base> {
        &self.base
    }

    // Multi-threaded binds are currently blocking. However, if they still report an error, we follow the single-threaded behavior
    // of capturing the backtrace. This may be changed as the threading model (#18) evolves.

    fn get(&self) -> RefGuard<'_, T> {
        let guard = self
            .user_instance
            .borrow()
            .unwrap_or_else(|e| super::bind_failed::<T>(e, &self.borrow_tracker));

        self.borrow_tracker.track_ref_borrow();
        guard
    }

    fn get_mut(&self) -> MutGuard<'_, T> {
        let guard = self
            .user_instance
            .borrow_mut()
            .unwrap_or_else(|e| super::bind_mut_failed::<T>(e, &self.borrow_tracker));

        self.borrow_tracker.track_mut_borrow();
        guard
    }

    fn get_inaccessible<'a: 'b, 'b>(
        &'a self,
        value: &'b mut Self::Instance,
    ) -> InaccessibleGuard<'b, T> {
        self.user_instance
            .make_inaccessible(value)
            .unwrap_or_else(|e| super::bug_inaccessible::<T>(e))
    }

    fn get_lifecycle(&self) -> Lifecycle {
        self.lifecycle.get()
    }

    fn set_lifecycle(&self, lifecycle: Lifecycle) {
        self.lifecycle.set(lifecycle)
    }
}

impl<T: GodotClass> StorageRefCounted for InstanceStorage<T> {
    fn on_inc_ref(&self) {
        super::log_inc_ref(self);
    }

    fn on_dec_ref(&self) {
        super::log_dec_ref(self);
    }
}

impl<T: GodotClass> Drop for InstanceStorage<T> {
    fn drop(&mut self) {
        super::log_drop(self);
    }
}
