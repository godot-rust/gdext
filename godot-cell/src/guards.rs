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
/// No other mutable borrows to the same value will be created while this guard exists.
#[derive(Debug)]
pub struct RefGuard<'a, T> {
    state: &'a Mutex<CellState<T>>,
    value: NonNull<T>,
}

impl<'a, T> RefGuard<'a, T> {
    /// Create a new `GdRef` guard which can be immutably dereferenced.
    ///
    /// # Safety
    ///
    /// It must be safe to call [`as_ref()`](NonNull::as_ref) on `value` for as long as the guard is not
    /// dropped.
    ///
    /// In particular you must ensure that:
    ///
    /// - The value behind the `value` pointer must be accessible for as long as the guard is not dropped.
    /// - No new mutable references to the same value can be created.
    /// - If there exist any other mutable references, then `value` must be derived from those references.
    /// - Any existing mutable references must stop accessing this value while this guard exists.
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
/// This prevents all other borrows of `value`, unless the `&mut` reference handed out from this guard is set
/// as non-aliasing by a call to [`GdCell::set_non_aliasing()`](crate::GdCell::set_non_aliasing).
#[derive(Debug)]
pub struct MutGuard<'a, T> {
    state: &'a Mutex<CellState<T>>,
    count: usize,
    value: NonNull<T>,
}

impl<'a, T> MutGuard<'a, T> {
    /// Create a new `GdMut` guard which can be mutably dereferenced.
    ///
    /// # Safety
    ///
    /// For as long as the guard lives:
    ///
    /// - It must be safe to call [`as_ref()`](NonNull::as_ref) on `value` when you have a shared reference to
    ///   the guard.
    /// - It must be safe to call [`as_mut()`](NonNull::as_mut) on `value` when you have a mutable reference
    ///   to the guard.
    ///
    /// In particular you must ensure that until the guard is set as non-aliasing via a call to
    /// [`GdCell::set_non_aliasing()`](crate::GdCell::set_non_aliasing), then:
    ///
    /// - The value behind the `value` pointer must be accessible for as long as the guard is not dropped.
    /// - No new references to `value` may be created.
    /// - If there exist any other mutable references, then `value` must be derived from those references.
    /// - Any existing mutable references must stop accessing this value while this guard exists.
    ///
    /// When the guard is set as non-aliasing, then:
    ///
    /// - Any references handed out by this guard must stop accessing the value.
    ///
    /// This can be satisfied by using `deref_mut` to get a `&mut T` to the value, then making that reference
    /// inaccessible for the duration that this guard is marked as non-aliasing. As the rust borrow checker
    /// will prevent anyone from then using this guard until that reference is dropped.
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

/// A guard that ensures a mutable reference is kept inaccessible until it is dropped. At which point the
/// borrow-state is set to prevent aliasing for the most recent borrow and the pointer in state is set to the
/// previous pointer.
#[derive(Debug)]
pub struct NonAliasingGuard<'a, T> {
    state: &'a Mutex<CellState<T>>,
    prev_ptr: NonNull<T>,
}

impl<'a, T> NonAliasingGuard<'a, T> {
    /// Create a new non-aliasing guard for `state`.
    pub(crate) fn new(state: &'a Mutex<CellState<T>>, prev_ptr: NonNull<T>) -> Self {
        Self { state, prev_ptr }
    }
}

impl<'a, T> Drop for NonAliasingGuard<'a, T> {
    fn drop(&mut self) {
        let mut state = self.state.lock().unwrap();
        state.borrow_state.unset_non_aliasing().unwrap();
        state.set_ptr(self.prev_ptr);
    }
}
