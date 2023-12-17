/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::ops::{Deref, DerefMut};
use std::ptr::NonNull;
use std::sync::Mutex;

use crate::CellState;

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Wraps a shared borrowed value of type `T`.
///
/// No mutable borrows to the same value can be created while this guard exists.
#[derive(Debug)]
pub struct RefGuard<'a, T> {
    /// The current state of borrows to the borrowed value.
    state: &'a Mutex<CellState<T>>,

    /// A pointer to the borrowed value.
    value: NonNull<T>,
}

impl<'a, T> RefGuard<'a, T> {
    /// Create a new `GdRef` guard which can be immutably dereferenced.
    ///
    /// # Safety
    ///
    /// While the returned guard exists you must ensure that:
    ///
    /// - It is safe to access the value behind the `value` pointer through a shared reference derived from
    ///   the `value` pointer.
    /// - No new mutable references to the same value can be created.
    /// - If there exist any other mutable references, then `value` must be derived from those references.
    /// - Any existing mutable references must stop accessing this value while this guard exists.
    ///
    /// These conditions ensure that it is safe to call [`as_ref()`](NonNull::as_ref) on `value` for as long
    /// as the returned guard exists.
    pub(crate) unsafe fn new(state: &'a Mutex<CellState<T>>, value: NonNull<T>) -> Self {
        Self { state, value }
    }
}

impl<'a, T> Deref for RefGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // SAFETY: It is safe to call `as_ref()` on value because of the safety invariants of `new`.
        unsafe { self.value.as_ref() }
    }
}

impl<'a, T> Drop for RefGuard<'a, T> {
    fn drop(&mut self) {
        self.state
            .lock()
            .unwrap()
            .borrow_state
            .decrement_shared()
            .unwrap();
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Wraps a mutably borrowed value of type `T`.
///
/// This prevents all other borrows of `value`, unless the `&mut` reference handed out from this guard is
/// made inaccessible by a call to [`GdCell::make_inaccessible()`](crate::GdCell::make_inaccessible).
#[derive(Debug)]
pub struct MutGuard<'a, T> {
    state: &'a Mutex<CellState<T>>,
    count: usize,
    value: NonNull<T>,
}

impl<'a, T> MutGuard<'a, T> {
    /// Create a new `MutGuard` guard which can be mutably dereferenced.
    ///
    /// # Safety
    ///
    /// While the returned guard exists and is accessible you must ensure that:
    ///
    /// - It is safe to access the value behind the `value` pointer through a shared or mutable reference
    ///   derived from the `value` pointer.
    /// - No new references to `value` may be created.
    /// - If there exist any other mutable references, then `value` must be derived from those references.
    /// - Any existing mutable references must stop accessing this value while this guard exists.
    ///
    /// To make a `MutGuard` inaccessible, you must pass a `&mut T` reference from this guard to
    /// [`GdCell::make_inaccessible()`](crate::GdCell::make_inaccessible).
    ///
    /// Together, these conditions ensure that it is safe to call [`as_ref()`](NonNull::as_ref) and
    /// [`as_mut()`](NonNull::as_mut) on `value` whenever we have a `&self` or `&mut self` reference to the
    /// guard.
    ///
    /// This is the case because:
    /// - [`GdCell`](super::GdCell) will not create any new references while this guard exists and is
    ///   accessible.
    /// - When it is marked as inaccessible it is impossible to have any `&self` or `&mut self` references to
    ///   this guard that can be used. Because we take in a `&mut self` reference with a lifetime `'a` and
    ///   return an [`InaccessibleGuard`] with a lifetime `'b` where `'a: 'b` which ensure that the
    ///   `&mut self` outlives that guard and cannot be used until the guard is dropped. And the rust
    ///   borrow-checker will prevent any new references from being made.
    /// - When it is made inaccessible, [`GdCell`](super::GdCell) will also ensure that any new references
    ///   are derived from this guard's `value` pointer, thus preventing `value` from being invalidated.
    pub(crate) unsafe fn new(
        state: &'a Mutex<CellState<T>>,
        count: usize,
        value: NonNull<T>,
    ) -> Self {
        Self {
            state,
            count,
            value,
        }
    }
}

impl<'a, T> Deref for MutGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        let count = self.state.lock().unwrap().borrow_state.mut_count();
        // This is just a best-effort error check. It should never be triggered.
        assert_eq!(
            self.count, count,
            "attempted to access the non-current mutable borrow. **this is a bug, please report it**"
        );

        // SAFETY:
        // It is safe to call `as_ref()` on value when we have a `&self` reference because of the safety
        // invariants of `new`.
        unsafe { self.value.as_ref() }
    }
}

impl<'a, T> DerefMut for MutGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        let count = self.state.lock().unwrap().borrow_state.mut_count();
        // This is just a best-effort error check. It should never be triggered.
        assert_eq!(
            self.count, count,
            "attempted to access the non-current mutable borrow. **this is a bug, please report it**"
        );

        // SAFETY:
        // It is safe to call `as_mut()` on value when we have a `&mut self` reference because of the safety
        // invariants of `new`.
        unsafe { self.value.as_mut() }
    }
}

impl<'a, T> Drop for MutGuard<'a, T> {
    fn drop(&mut self) {
        self.state
            .lock()
            .unwrap()
            .borrow_state
            .decrement_mut()
            .unwrap();
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// A guard that ensures a mutable reference is kept inaccessible until it is dropped.
///
/// The current reference is stored in the guard and we push a new reference to `state` on creation. We then
/// undo this upon dropping the guard.
///
/// This ensure that any new references are derived from the new reference we pass in, and when this guard is
/// dropped we reset it to the previous reference.
#[derive(Debug)]
pub struct InaccessibleGuard<'a, T> {
    state: &'a Mutex<CellState<T>>,
    prev_ptr: NonNull<T>,
}

impl<'a, T> InaccessibleGuard<'a, T> {
    /// Create a new inaccessible guard for `state`.
    ///
    /// Since `'b` must outlive `'a`, we cannot have any other references aliasing `new_ref` while this
    /// guard exists.
    pub(crate) fn new<'b>(
        state: &'a Mutex<CellState<T>>,
        new_ref: &'b mut T,
    ) -> Result<Self, Box<dyn std::error::Error>>
    where
        'a: 'b,
    {
        let mut guard = state.lock().unwrap();

        let current_ptr = guard.get_ptr();
        let new_ptr = NonNull::from(new_ref);

        if current_ptr != new_ptr {
            // it is likely not unsound for this to happen, but it's unexpected
            return Err("wrong reference passed in".into());
        }

        guard.borrow_state.set_inaccessible()?;
        let prev_ptr = guard.get_ptr();
        guard.set_ptr(new_ptr);

        Ok(Self { state, prev_ptr })
    }
}

impl<'a, T> Drop for InaccessibleGuard<'a, T> {
    fn drop(&mut self) {
        let mut state = self.state.lock().unwrap();
        state.borrow_state.unset_inaccessible().unwrap();
        state.set_ptr(self.prev_ptr);
    }
}
