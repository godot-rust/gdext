/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
use std::cell::UnsafeCell;
use std::ops::{Deref, DerefMut};
use std::ptr::NonNull;

use crate::cell::CellState;

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Wraps a shared borrowed value of type `T`.
///
/// No mutable borrows to the same value can be created while this guard exists.
#[derive(Debug)]
pub struct RefGuard<'a, T> {
    /// The current state of borrows to the borrowed value.
    state: &'a UnsafeCell<CellState<T>>,

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
    pub(crate) unsafe fn new(state: &'a UnsafeCell<CellState<T>>, value: NonNull<T>) -> Self {
        Self { state, value }
    }
}

impl<T> Deref for RefGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // SAFETY: It is safe to call `as_ref()` on value because of the safety invariants of `new`.
        unsafe { self.value.as_ref() }
    }
}

impl<T> Drop for RefGuard<'_, T> {
    fn drop(&mut self) {
        // SAFETY: There is no other active reference to the state, and it is ensured that RefGuard is alive at least as long as the reference to the state.
        unsafe { CellState::borrow_state(self.state) }
            .decrement_shared()
            .unwrap();
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Wraps a mutably borrowed value of type `T`.
///
/// This prevents all other borrows of `value` while this guard is accessible. To make this guard
/// inaccessible, use [`GdCell::make_inaccessible()`](crate::panicking::GdCell::make_inaccessible) on a mutable
/// reference handed out by this guard.
#[derive(Debug)]
pub struct MutGuard<'a, T> {
    state: &'a UnsafeCell<CellState<T>>,
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
    /// - When it is made inaccessible it is impossible to have any `&self` or `&mut self` references to this
    ///   guard that can be used. Because we take in a `&mut self` reference with a lifetime `'a` and return
    ///   an [`InaccessibleGuard`] with a lifetime `'b` where `'a: 'b` which ensure that the `&mut self`
    ///   outlives that guard and cannot be used until the guard is dropped. And the rust borrow-checker will
    ///   prevent any new references from being made.
    /// - When it is made inaccessible, [`GdCell`](super::GdCell) will also ensure that any new references
    ///   are derived from this guard's `value` pointer, thus preventing `value` from being invalidated.
    pub(crate) unsafe fn new(
        state: &'a UnsafeCell<CellState<T>>,
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

impl<T> Deref for MutGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // SAFETY: There can't be any other active reference to CellState.
        let count = unsafe { CellState::borrow_state(self.state) }.mut_count();
        // This is just a best-effort error check. It should never be triggered.
        assert_eq!(
            self.count,
            count,
            "\
            attempted to access a non-current mutable borrow of type: `{}`. \n\
            current count: {}\n\
            value pointer: {:p}\n\
            attempted access count: {}\n\
            **this is a bug, please report it**\
            ",
            std::any::type_name::<T>(),
            self.count,
            self.value,
            count
        );

        // SAFETY: It is safe to call `as_ref()` on value when we have a `&self` reference because of the
        // safety invariants of `new`.
        unsafe { self.value.as_ref() }
    }
}

impl<T> DerefMut for MutGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: There can't be any other active reference to CellState.
        let count = unsafe { CellState::borrow_state(self.state) }.mut_count();
        // This is just a best-effort error check. It should never be triggered.
        assert_eq!(
            self.count,
            count,
            "\
            attempted to access a non-current mutable borrow of type: `{}`. \n\
            current count: {}\n\
            value pointer: {:p}\n\
            attempted access count: {}\n\
            **this is a bug, please report it**\
            ",
            std::any::type_name::<T>(),
            self.count,
            self.value,
            count
        );

        // SAFETY:
        // It is safe to call `as_mut()` on value when we have a `&mut self` reference because of the safety
        // invariants of `new`.
        unsafe { self.value.as_mut() }
    }
}

impl<T> Drop for MutGuard<'_, T> {
    fn drop(&mut self) {
        // SAFETY: It is ensured that MutGuard is exclusive and alive at least as long as the reference to the state.
        unsafe { CellState::borrow_state(self.state) }
            .decrement_mut()
            .unwrap();
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// A guard that ensures a mutable reference is kept inaccessible until it is dropped.
///
/// We store the current reference in the guard upon creation, and push a new reference to `state` on
/// creation. When the guard is dropped, `state`'s pointer is reset to the original pointer.
///
/// This ensures that any new references are derived from the new reference we pass in, and when this guard
/// is dropped, it resets the state to what it was before, as if this guard never existed.
#[derive(Debug)]
pub struct InaccessibleGuard<'a, T> {
    state: &'a UnsafeCell<CellState<T>>,
    stack_depth: usize,
    prev_ptr: NonNull<T>,
}

impl<'a, T> InaccessibleGuard<'a, T> {
    /// Create a new inaccessible guard for `state`.
    ///
    /// Since `'b` must outlive `'a`, we cannot have any other references aliasing `new_ref` while this
    /// guard exists. So this guard ensures that the guard that handed out `new_ref` is inaccessible while
    /// this guard exists.
    ///
    /// Will error if:
    /// - There is currently no accessible mutable borrow.
    /// - There are any shared references.
    /// - `new_ref` is not equal to the pointer in `state`.
    pub(crate) fn new<'b>(
        state: &'a UnsafeCell<CellState<T>>,
        new_ref: &'b mut T,
    ) -> Result<Self, Box<dyn std::error::Error>>
    where
        'a: 'b,
    {
        // SAFETY: There can be only one active reference to the cell state at a given time.
        let cell_state = unsafe { state.get().as_mut() }.unwrap();
        let current_ptr = cell_state.get_ptr();
        let new_ptr = NonNull::from(new_ref);

        if current_ptr != new_ptr {
            // it is likely not unsound for this to happen, but it's unexpected
            return Err("wrong reference passed in".into());
        }

        cell_state.borrow_state.set_inaccessible()?;
        let prev_ptr = cell_state.get_ptr();
        let stack_depth = cell_state.push_ptr(new_ptr);

        Ok(Self {
            state,
            stack_depth,
            prev_ptr,
        })
    }

    /// Single implementation of drop-logic for use in both drop implementations.
    fn perform_drop(state: &'a UnsafeCell<CellState<T>>, prev_ptr: NonNull<T>, stack_depth: usize) {
        let state = unsafe { state.get().as_mut() }.unwrap();
        if state.stack_depth != stack_depth {
            state
                .borrow_state
                .poison("cannot drop inaccessible guards in the wrong order")
                .unwrap();
        }
        state.borrow_state.unset_inaccessible().unwrap();
        state.pop_ptr(prev_ptr);
    }

    /// Returns `true` if guard can be safely dropped, i.e.:
    ///
    /// - Guard is being released in correct order.
    /// - There is no accessible mutable reference to underlying value.
    /// - There are no shared references to underlying value.
    #[doc(hidden)]
    pub fn can_drop(&self) -> bool {
        let state = unsafe { self.state.get().as_mut() }.unwrap();
        state.borrow_state.may_unset_inaccessible() || state.stack_depth == self.stack_depth
    }

    /// Drop self if possible, otherwise returns self again.
    ///
    /// Used currently in the mock-tests, as we need a thread safe way to drop self. Using the normal drop
    /// logic may poison state, however it should not cause any UB either way.
    #[doc(hidden)]
    pub fn try_drop(self) -> Result<(), Self> {
        if !self.can_drop() {
            return Err(self);
        }

        let manual = std::mem::ManuallyDrop::new(self);
        Self::perform_drop(manual.state, manual.prev_ptr, manual.stack_depth);

        Ok(())
    }
}

impl<T> Drop for InaccessibleGuard<'_, T> {
    fn drop(&mut self) {
        // Default behavior of drop-logic simply panics and poisons the cell on failure. This is appropriate
        // for single-threaded code where no errors should happen here.
        Self::perform_drop(self.state, self.prev_ptr, self.stack_depth);
    }
}
