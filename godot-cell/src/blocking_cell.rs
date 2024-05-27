/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::collections::HashMap;
use std::error::Error;
use std::pin::Pin;
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::thread;

use crate::blocking_guards::{MutGuardBlocking, RefGuardBlocking};
use crate::cell::GdCellInner;
use crate::guards::InaccessibleGuard;

/// Blocking version of [`panicking::GdCell`](crate::panicking::GdCell) for multi-threaded usage.
///
/// This version of GdCell blocks the current thread if it does not yet hold references to the cell.
///
/// For more details on when threads are being blocked see [`Self::borrow`] and [`Self::borrow_mut`].
///
/// See [`panicking::GdCell`](crate::panicking::GdCell) for more details on the base concept of this type.
pub struct GdCellBlocking<T> {
    inner: Pin<Box<GdCellInner<T>>>,
    thread_tracker: Arc<Mutex<ThreadTracker>>,
    immut_condition: Arc<Condvar>,
    mut_condition: Arc<Condvar>,
}

impl<T> GdCellBlocking<T> {
    pub fn new(value: T) -> Self {
        Self {
            inner: GdCellInner::new(value),
            thread_tracker: Arc::default(),
            immut_condition: Arc::new(Condvar::new()),
            mut_condition: Arc::new(Condvar::new()),
        }
    }

    /// Returns a new shared reference to the contents of the cell.
    ///
    /// Fails if an accessible mutable reference exists on the current thread.
    ///
    /// Blocks if another thread currently holds a mutable reference.
    pub fn borrow(&self) -> Result<RefGuardBlocking<'_, T>, Box<dyn Error>> {
        let mut tracker_guard = self.thread_tracker.lock().unwrap();

        if self.inner.as_ref().is_currently_mutably_bound()
            && !tracker_guard.current_thread_has_mut_ref()
        {
            // Block current thread until borrow becomes available.
            tracker_guard = self.block_immut(tracker_guard);
        }

        let inner_guard = self.inner.as_ref().borrow()?;

        tracker_guard.increment_current_thread_shared_count();

        Ok(RefGuardBlocking::new(
            inner_guard,
            self.mut_condition.clone(),
            self.thread_tracker.clone(),
        ))
    }

    /// Returns a new mutable reference to the contents of the cell.
    ///
    /// Fails if an accessible mutable reference, or a shared reference exists on the current thread.
    ///
    /// Blocks if another thread currently holds a mutable reference, or if another thread holds immutable references but the current thread
    /// doesn't.
    pub fn borrow_mut(&self) -> Result<MutGuardBlocking<'_, T>, Box<dyn Error>> {
        let mut tracker_guard = self.thread_tracker.lock().unwrap();

        if self.inner.as_ref().is_currently_bound()
            && tracker_guard.current_thread_shared_count() == 0
            && !tracker_guard.current_thread_has_mut_ref()
        {
            // Block current thread until borrow becomes available.
            tracker_guard = self.block_mut(tracker_guard);
        }

        let inner_guard = self.inner.as_ref().borrow_mut()?;

        tracker_guard.mut_thread = thread::current().id();

        Ok(MutGuardBlocking::new(
            inner_guard,
            self.mut_condition.clone(),
            self.immut_condition.clone(),
        ))
    }

    /// Make the current mutable borrow inaccessible, thus freeing the value up to be reborrowed again.
    ///
    /// Will error if:
    /// - There is currently no accessible mutable borrow.
    /// - There are any shared references.
    /// - `current_ref` is not equal to the pointer in `self.inner.state`.
    pub fn make_inaccessible<'cell, 'val>(
        &'cell self,
        current_ref: &'val mut T,
    ) -> Result<InaccessibleGuard<'val, T>, Box<dyn Error>>
    where
        'cell: 'val,
    {
        self.inner.as_ref().make_inaccessible(current_ref)
    }

    /// Returns `true` if there are any mutable or shared references, regardless of whether the mutable
    /// references are accessible or not.
    ///
    /// In particular this means that it is safe to destroy this cell and the value contained within, as no
    /// references can exist that can reference this cell.
    ///
    /// Keep in mind that in multithreaded code it is still possible for this to return true, and then the
    /// cell hands out a new borrow before it is destroyed. So we still need to ensure that this cannot
    /// happen at the same time.
    pub fn is_currently_bound(&self) -> bool {
        self.inner.as_ref().is_currently_bound()
    }

    /// Blocks the current thread until all mutable and shared references have been dropped.
    fn block_mut<'a>(
        &self,
        mut tracker_guard: MutexGuard<'a, ThreadTracker>,
    ) -> MutexGuard<'a, ThreadTracker> {
        while self.inner.as_ref().is_currently_bound() {
            tracker_guard = self.mut_condition.wait(tracker_guard).unwrap();
        }

        tracker_guard
    }

    /// Blocks the current thread until all mutable references have been dropped.
    fn block_immut<'a>(
        &self,
        mut tracker_guard: MutexGuard<'a, ThreadTracker>,
    ) -> MutexGuard<'a, ThreadTracker> {
        while self.inner.as_ref().is_currently_mutably_bound() {
            tracker_guard = self.immut_condition.wait(tracker_guard).unwrap();
        }

        tracker_guard
    }
}

/// Holds the reference count and the currently mutable thread.
#[derive(Debug)]
pub(crate) struct ThreadTracker {
    /// Thread ID of the thread that currently can hold the mutable reference.
    mut_thread: thread::ThreadId,

    /// Shared reference count per thread.
    shared_counts: HashMap<thread::ThreadId, usize>,
}

impl Default for ThreadTracker {
    fn default() -> Self {
        Self {
            mut_thread: thread::current().id(),
            shared_counts: HashMap::new(),
        }
    }
}

impl ThreadTracker {
    /// Number of shared references in the current thread.
    pub fn current_thread_shared_count(&self) -> usize {
        *self
            .shared_counts
            .get(&thread::current().id())
            .unwrap_or(&0)
    }

    /// Increments the shared reference count in the current thread.
    pub fn increment_current_thread_shared_count(&mut self) {
        self.shared_counts
            .entry(thread::current().id())
            .and_modify(|count| *count += 1)
            .or_insert(1);
    }

    /// Decrements the shared reference count in the current thread.
    pub fn decrement_current_thread_shared_count(&mut self) {
        let thread_id = thread::current().id();
        let entry = self.shared_counts.get_mut(&thread_id);

        debug_assert!(
            entry.is_some(),
            "No shared reference count exists for {thread_id:?}."
        );

        let Some(count) = entry else {
            return;
        };

        *count -= 1;
    }

    /// Returns if the current thread can hold the mutable reference.
    pub fn current_thread_has_mut_ref(&self) -> bool {
        self.mut_thread == thread::current().id()
    }
}
