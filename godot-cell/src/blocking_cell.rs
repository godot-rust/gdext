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

use crate::blocking_guards::{InaccessibleGuardBlocking, MutGuardBlocking, RefGuardBlocking};
use crate::cell::GdCellInner;

/// Blocking version of [`panicking::GdCell`](crate::panicking::GdCell) for multithreaded usage.
///
/// This version of GdCell blocks the current thread if it does not yet hold references to the cell.
/// Since `GdCellInner` isn't thread-safe by itself, any access to `inner` must be guarded by locking the `thread_tracker`.
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

        let should_claim_mut = !self.is_currently_bound();

        let inner_guard = self.inner.as_ref().borrow()?;

        tracker_guard.increment_current_thread_shared_count();

        // The ThreadTracker::mut_thread is always set to some ThreadId to avoid additional tracking overhead, but this causes an edge case:
        // 1. mut_thread is initialized with the current Thread 1 (usually main thread).
        // 2. Thread 2 acquires an immutable borrow from the cell.
        // 3. Thread 1 attempts to acquire a mutable borrow from the cell.
        // 4. No immutable borrow exists on Thread 1, but the thread is assigned as the mut_thread; no blocking occurs.
        // 5. The mutable borrow fails and panics because there is already an immutable borrow on Thread 1.
        //
        // Solution:
        // 1. Always reassign mut_thread to the current ThreadId (i.e. Thread 2) when the first immutable borrow is acquired.
        // 2. Thread 1 will now block on mutable borrows because it is not the mut_thread.
        // 3. Thread 2 should never block, as it already holds an immutable borrow.
        if should_claim_mut {
            tracker_guard.claim_mut_ref();
        }

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

        tracker_guard.claim_mut_ref();

        Ok(MutGuardBlocking::new(
            inner_guard,
            self.mut_condition.clone(),
            self.immut_condition.clone(),
            self.thread_tracker.clone(),
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
    ) -> Result<InaccessibleGuardBlocking<'val, T>, Box<dyn Error>>
    where
        'cell: 'val,
    {
        let _tracker_guard = self.thread_tracker.lock().unwrap();
        let inner = self.inner.as_ref().make_inaccessible(current_ref)?;
        let inaccessible = InaccessibleGuardBlocking::new(inner, self.thread_tracker.clone());
        Ok(inaccessible)
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

// SAFETY:
// - `T` must not be `Sync`, and the only way to access the underlying `T` is via `GdCellBlocking`.
// - It must be ensured that `GdCellInner`, which holds `T`, cannot be accessed from multiple threads simultaneously while handing out guards.
// The current implementation ensures this by locking the `thread_tracker`.
unsafe impl<T: Send> Sync for GdCellBlocking<T> {}

/// Holds the reference count and the currently mutable thread.
#[derive(Debug)]
pub(crate) struct ThreadTracker {
    /// Thread ID of the thread that currently can hold the mutable reference.
    ///
    /// This is not an Option, contrary to what one might expect. Making this an option would require reliable knowledge that not a single
    /// MutGuardBlocking exists before setting it to None. This would require tracking a count of mutable borrows. Instead, we always set
    /// the field to an acceptable value and avoid the overhead.
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

    /// Claims the mutable reference for the current thread.
    fn claim_mut_ref(&mut self) {
        self.mut_thread = thread::current().id();
    }
}
