/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::cell::UnsafeCell;
use std::error::Error;
use std::marker::PhantomPinned;
use std::pin::Pin;
use std::ptr::NonNull;
use std::sync::Mutex;

use crate::borrow_state::BorrowState;
use crate::guards::{InaccessibleGuard, MutGuard, RefGuard};

/// A cell which can hand out new `&mut` references to its value even when one already exists. As long as
/// any such pre-existing references have been handed back to the cell first, and no shared references exist.
pub struct GdCell<T>(Pin<Box<GdCellInner<T>>>);

impl<T> GdCell<T> {
    /// Creates a new cell storing `value`.
    pub fn new(value: T) -> Self {
        Self(GdCellInner::new(value))
    }

    /// Returns a new shared reference to the contents of the cell.
    ///
    /// Fails if an accessible mutable reference exists.
    pub fn borrow(&self) -> Result<RefGuard<'_, T>, Box<dyn Error>> {
        self.0.as_ref().borrow()
    }

    /// Returns a new mutable reference to the contents of the cell.
    ///
    /// Fails if an accessible mutable reference exists, or a shared reference exists.
    pub fn borrow_mut(&self) -> Result<MutGuard<'_, T>, Box<dyn Error>> {
        self.0.as_ref().borrow_mut()
    }

    /// Make the current mutable borrow inaccessible, thus freeing the value up to be reborrowed again.
    ///
    /// Will error if:
    /// - There is currently no accessible mutable borrow.
    /// - There are any shared references.
    /// - `current_ref` is not equal to the pointer in `self.inner.state`.
    pub fn make_inaccessible<'cell, 'val>(
        &'cell self,
        original_ref: &'val mut T,
    ) -> Result<InaccessibleGuard<'val, T>, Box<dyn Error>>
    where
        'cell: 'val,
    {
        self.0.as_ref().make_inaccessible(original_ref)
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
        self.0.as_ref().is_currently_bound()
    }
}

/// Internals of [`GdCell`].
///
/// This cell must be pinned to be usable, as it stores self-referential pointers. The [`GdCell`] type abstracts this detail away from
/// the public type.
// TODO: consider not using `Mutex`
#[derive(Debug)]
pub(crate) struct GdCellInner<T> {
    /// The mutable state of this cell.
    pub(crate) state: Mutex<CellState<T>>,
    /// The actual value we're handing out references to, uses `UnsafeCell` as we're passing out `&mut`
    /// references to its contents even when we only have a `&` reference to the cell.
    value: UnsafeCell<T>,
    /// We don't want to be able to take `GdCell` out of a pin, so `GdCell` cannot implement `Unpin`.
    _pin: PhantomPinned,
}

impl<T> GdCellInner<T> {
    /// Creates a new cell storing `value`.
    pub fn new(value: T) -> Pin<Box<Self>> {
        let cell = Box::pin(Self {
            state: Mutex::new(CellState::new()),
            value: UnsafeCell::new(value),
            _pin: PhantomPinned,
        });

        cell.state.lock().unwrap().initialize_ptr(&cell.value);

        cell
    }

    /// Returns a new shared reference to the contents of the cell.
    ///
    /// Fails if an accessible mutable reference exists.
    pub fn borrow(self: Pin<&Self>) -> Result<RefGuard<'_, T>, Box<dyn Error>> {
        let mut state = self.state.lock().unwrap();
        state.borrow_state.increment_shared()?;

        // SAFETY: `increment_shared` succeeded, therefore there cannot currently be any accessible mutable
        // references.
        unsafe { Ok(RefGuard::new(&self.get_ref().state, state.get_ptr())) }
    }

    /// Returns a new mutable reference to the contents of the cell.
    ///
    /// Fails if an accessible mutable reference exists, or a shared reference exists.
    pub fn borrow_mut(self: Pin<&Self>) -> Result<MutGuard<'_, T>, Box<dyn Error>> {
        let mut state = self.state.lock().unwrap();
        state.borrow_state.increment_mut()?;
        let count = state.borrow_state.mut_count();
        let value = state.get_ptr();

        // SAFETY: `increment_mut` succeeded, therefore any existing mutable references are inaccessible.
        // Additionally, no new references can be created, unless the returned guard is made inaccessible.
        //
        // This is the case because the only way for a new `GdMut` or `GdRef` to be made after this is for
        // either this guard to be dropped or `make_inaccessible` to be called and succeed.
        //
        // If this guard is dropped, then we don't need to worry.
        //
        // If `make_inaccessible` is called and succeeds, then a mutable reference from this guard is passed
        // in. In which case, we cannot use this guard again until the resulting inaccessible guard is
        // dropped.
        unsafe { Ok(MutGuard::new(&self.get_ref().state, count, value)) }
    }

    /// Make the current mutable borrow inaccessible, thus freeing the value up to be reborrowed again.
    ///
    /// Will error if:
    /// - There is currently no accessible mutable borrow.
    /// - There are any shared references.
    /// - `current_ref` is not equal to the pointer in `self.state`.
    pub fn make_inaccessible<'cell: 'val, 'val>(
        self: Pin<&'cell Self>,
        current_ref: &'val mut T,
    ) -> Result<InaccessibleGuard<'val, T>, Box<dyn Error>> {
        InaccessibleGuard::new(&self.get_ref().state, current_ref)
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
    pub fn is_currently_bound(self: Pin<&Self>) -> bool {
        let state = self.state.lock().unwrap();

        state.borrow_state.shared_count() > 0 || state.borrow_state.mut_count() > 0
    }

    /// Similar to [`Self::is_currently_bound`] but only counts mutable references and ignores shared references.
    pub(crate) fn is_currently_mutably_bound(self: Pin<&Self>) -> bool {
        let state = self.state.lock().unwrap();

        state.borrow_state.mut_count() > 0
    }
}

// SAFETY: `T` is Sync, so we can return references to it on different threads.
// It is also Send, so we can return mutable references to it on different threads.
// Additionally, all internal state is synchronized via a mutex, so we won't have race conditions when trying to use it from multiple threads.
unsafe impl<T: Send + Sync> Sync for GdCellInner<T> {}

/// Mutable state of the `GdCell`, bundled together to make it easier to avoid deadlocks when locking the
/// mutex.
#[derive(Debug)]
pub(crate) struct CellState<T> {
    /// Tracking the borrows this cell has. This ensures relevant invariants are upheld.
    pub(crate) borrow_state: BorrowState,

    /// Current pointer to the value.
    ///
    /// This will always be non-null after initialization.
    ///
    /// When a reference is handed to a cell to enable reborrowing, then this pointer is set to that
    /// reference.
    ///
    /// We always generate new pointer based off of the pointer currently in this field, to ensure any new
    /// references are derived from the most recent `&mut` reference.
    // TODO: Consider using `NonNull<T>` instead.
    ptr: *mut T,

    /// How many pointers have been handed out.
    ///
    /// This is used to ensure that the pointers are not replaced in the wrong order.
    pub(crate) stack_depth: usize,
}

impl<T> CellState<T> {
    /// Create a new uninitialized state. Use [`initialize_ptr()`](CellState::initialize_ptr()) to initialize
    /// it.
    fn new() -> Self {
        Self {
            borrow_state: BorrowState::new(),
            ptr: std::ptr::null_mut(),
            stack_depth: 0,
        }
    }

    /// Initialize the pointer if it is `None`.
    fn initialize_ptr(&mut self, value: &UnsafeCell<T>) {
        if self.ptr.is_null() {
            self.ptr = value.get();
            assert!(!self.ptr.is_null());
        } else {
            panic!("Cannot initialize pointer as it is already initialized.")
        }
    }

    /// Returns the current pointer. Panics if uninitialized.
    pub(crate) fn get_ptr(&self) -> NonNull<T> {
        NonNull::new(self.ptr).unwrap()
    }

    /// Push a pointer to this state.
    pub(crate) fn push_ptr(&mut self, new_ptr: NonNull<T>) -> usize {
        self.ptr = new_ptr.as_ptr();
        self.stack_depth += 1;
        self.stack_depth
    }

    /// Pop a pointer to this state, resetting it to the given old pointer.
    pub(crate) fn pop_ptr(&mut self, old_ptr: NonNull<T>) -> usize {
        self.ptr = old_ptr.as_ptr();
        self.stack_depth -= 1;
        self.stack_depth
    }
}

#[cfg(test)] #[cfg_attr(published_docs, doc(cfg(test)))]
mod test {
    use super::*;

    #[test]
    fn prevent_mut_mut() {
        const VAL: i32 = -451431556;
        let cell = GdCell::new(VAL);
        let guard1 = cell.borrow_mut().unwrap();
        let guard2 = cell.borrow_mut();

        assert_eq!(*guard1, VAL);
        assert!(guard2.is_err());
        std::mem::drop(guard1);
    }

    #[test]
    fn prevent_mut_shared() {
        const VAL: i32 = 13512;
        let cell = GdCell::new(VAL);
        let guard1 = cell.borrow_mut().unwrap();
        let guard2 = cell.borrow();

        assert_eq!(*guard1, VAL);
        assert!(guard2.is_err());
        std::mem::drop(guard1);
    }

    #[test]
    fn prevent_shared_mut() {
        const VAL: i32 = 99;
        let cell = GdCell::new(VAL);
        let guard1 = cell.borrow().unwrap();
        let guard2 = cell.borrow_mut();

        assert_eq!(*guard1, VAL);
        assert!(guard2.is_err());
        std::mem::drop(guard1);
    }

    #[test]
    fn allow_shared_shared() {
        const VAL: i32 = 10;
        let cell = GdCell::new(VAL);
        let guard1 = cell.borrow().unwrap();
        let guard2 = cell.borrow().unwrap();

        assert_eq!(*guard1, VAL);
        assert_eq!(*guard2, VAL);
        std::mem::drop(guard1);
    }

    #[test]
    fn allow_inaccessible_mut_mut() {
        const VAL: i32 = 23456;
        let cell = GdCell::new(VAL);

        let mut guard1 = cell.borrow_mut().unwrap();
        let mut1 = &mut *guard1;
        assert_eq!(*mut1, VAL);
        *mut1 = VAL + 50;

        let inaccessible_guard = cell.make_inaccessible(mut1).unwrap();

        let mut guard2 = cell.borrow_mut().unwrap();
        let mut2 = &mut *guard2;
        assert_eq!(*mut2, VAL + 50);
        *mut2 = VAL - 30;
        drop(guard2);

        drop(inaccessible_guard);

        assert_eq!(*mut1, VAL - 30);
        *mut1 = VAL - 5;

        drop(guard1);

        let guard3 = cell.borrow().unwrap();
        assert_eq!(*guard3, VAL - 5);
    }

    #[test]
    fn different_inaccessible() {
        const VAL1: i32 = 23456;
        const VAL2: i32 = 11111;
        let cell1 = GdCell::new(VAL1);
        let cell2 = GdCell::new(VAL2);

        let mut guard1 = cell1.borrow_mut().unwrap();
        let mut1 = &mut *guard1;

        assert_eq!(*mut1, VAL1);
        *mut1 = VAL1 + 10;

        let mut guard2 = cell2.borrow_mut().unwrap();
        let mut2 = &mut *guard2;

        assert_eq!(*mut2, VAL2);
        *mut2 = VAL2 + 10;

        let inaccessible_guard = cell1
            .make_inaccessible(mut2)
            .expect_err("should not allow different references");

        drop(inaccessible_guard);

        drop(guard1);
        drop(guard2);
    }
}
