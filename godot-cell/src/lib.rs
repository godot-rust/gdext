/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! A re-entrant cell implementation which allows for `&mut` references to be re-taken even while `&mut`
//! references still exist.
//!
//! This is done by ensuring any existing `&mut` references cannot alias the new reference, and that the new
//! reference is derived from the previous one.

mod borrow_state;
mod guards;

use std::cell::UnsafeCell;
use std::error::Error;
use std::marker::PhantomPinned;
use std::pin::Pin;
use std::ptr::NonNull;
use std::sync::Mutex;

use borrow_state::BorrowState;
pub use guards::{MutGuard, NonAliasingGuard, RefGuard};

/// A cell which can hand out new `&mut` references to it's value even when one already exists, as long as
/// any pre-existing such references have been handed back to the cell first, and no shared references exist.
///
/// This cell must be pinned to be usable, as it stores self-referential pointers.
// TODO: consider not using `Mutex`
#[derive(Debug)]
pub struct GdCell<T> {
    /// The mutable state of this cell.
    state: Mutex<CellState<T>>,
    /// The actual value we're handing out references to, uses `UnsafeCell` as we're passing out `&mut`
    /// references to its contents even when we only have a `&` reference to the cell.
    value: UnsafeCell<T>,
    /// We dont want to be able to take `GdCell` out of a pin, so `GdCell` cannot implement `Unpin`.
    _pin: PhantomPinned,
}

impl<T> GdCell<T> {
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
    /// Fails if an aliasing mutable reference exists.
    pub fn borrow(self: Pin<&Self>) -> Result<RefGuard<'_, T>, Box<dyn Error>> {
        let mut state = self.state.lock().unwrap();
        state.borrow_state.increment_shared()?;

        // SAFETY:
        // `increment_shared` succeeded, therefore there cannot currently be any aliasing mutable references.
        unsafe { Ok(RefGuard::new(&self.get_ref().state, state.get_ptr())) }
    }

    /// Returns a new shared reference to the contents of the cell.
    ///
    /// Fails if an aliasing mutable reference exists, or a shared reference exists.
    pub fn borrow_mut(self: Pin<&Self>) -> Result<MutGuard<'_, T>, Box<dyn Error>> {
        let mut state = self.state.lock().unwrap();
        state.borrow_state.increment_mut()?;
        let count = state.borrow_state.mut_count();
        let value = state.get_ptr();

        // SAFETY:
        // `increment_mut` succeeded, therefore any existing mutable references do not alias, and no new
        // references may be made unless this one is guaranteed not to alias those.
        //
        // This is the case because the only way for a new `GdMut` or `GdRef` to be made after this, is for
        // either this guard to be dropped or `set_non_aliasing` to be called.
        //
        // If this guard is dropped, then we dont need to worry.
        //
        // If `set_non_aliasing` is called, then either a mutable reference from this guard is passed in.
        // In which case, we cannot use this guard again until the resulting non-aliasing guard is dropped.
        //
        // We cannot pass in a different mutable reference, since `set_non_aliasing` ensures any references
        // matches the ones this one would return. And only one mutable reference to the same value can exist
        // since we cannot have any other aliasing mutable references around to pass in.
        unsafe { Ok(MutGuard::new(&self.get_ref().state, count, value)) }
    }

    /// Set the current mutable borrow as not aliasing any other references.
    ///
    /// Will error if there is no current possibly aliasing mutable borrow, or if there are any shared
    /// references.
    pub fn set_non_aliasing<'a, 'b>(
        self: Pin<&'a Self>,
        current_ref: &'b mut T,
    ) -> Result<NonAliasingGuard<'b, T>, Box<dyn Error>>
    where
        'a: 'b,
    {
        let mut state = self.state.lock().unwrap();
        let current_ptr = state.get_ptr();
        let ptr = NonNull::from(current_ref);

        if current_ptr != ptr {
            // it is likely not unsound for this to happen, but it's unexpected
            return Err("wrong reference passed in".into());
        }

        state.borrow_state.set_non_aliasing()?;
        let old_ptr = state.get_ptr();
        state.set_ptr(ptr);

        Ok(NonAliasingGuard::new(&self.get_ref().state, old_ptr))
    }

    /// Returns `true` if there are any mutable or shared references, regardless of whether the mutable
    /// references are aliasing or not.
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
}

// SAFETY:
// `T` is sync so we can return references to it on different threads.
// Additionally all internal state is synchronized via a mutex, so we wont have race conditions when trying
// to use it from multiple threads.
unsafe impl<T: Sync> Sync for GdCell<T> {}

/// Mutable state of the `GdCell`, bundled together to make it easier to avoid deadlocks when locking the
/// mutex.
#[derive(Debug)]
struct CellState<T> {
    /// Tracking the borrows this cell has. This ensures relevant invariants are upheld.
    borrow_state: BorrowState,
    /// Current pointer to the value.
    ///
    /// This is initialized upon first usage, as we cannot construct the cell pinned in general.
    ///
    /// When a reference is handed to a cell to enable re-entrancy, then this pointer is set to that
    /// reference.
    ///
    /// We always generate new pointer based off of the reference currently in this field, to ensure any new
    /// references are derived from the most recent `&mut` reference.
    ptr: Option<NonNull<T>>,
}

impl<T> CellState<T> {
    /// Create a new uninitialized state. Use [`initialize_ptr()`](CellState::initialize_ptr()) to initialize
    /// it.
    fn new() -> Self {
        Self {
            borrow_state: BorrowState::new(),
            ptr: None,
        }
    }

    /// Initialize the pointer if it is `None`.
    fn initialize_ptr(&mut self, value: &UnsafeCell<T>) {
        if self.ptr.is_none() {
            self.set_ptr(NonNull::new(value.get()).unwrap());
        } else {
            panic!("Cannot initialize pointer as it is already initialized.")
        }
    }

    /// Returns the current pointer. Panics if uninitialized.
    fn get_ptr(&self) -> NonNull<T> {
        self.ptr.unwrap()
    }

    /// Set the current pointer to the new pointer.
    fn set_ptr(&mut self, new_ptr: NonNull<T>) {
        self.ptr = Some(new_ptr);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn prevent_mut_mut() {
        const VAL: i32 = -451431556;
        let cell = GdCell::new(VAL);
        let cell = cell.as_ref();
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
        let cell = cell.as_ref();
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
        let cell = cell.as_ref();
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
        let cell = cell.as_ref();
        let guard1 = cell.borrow().unwrap();
        let guard2 = cell.borrow().unwrap();

        assert_eq!(*guard1, VAL);
        assert_eq!(*guard2, VAL);
        std::mem::drop(guard1);
    }

    #[test]
    fn allow_non_aliasing_mut_mut() {
        const VAL: i32 = 23456;
        let cell = GdCell::new(VAL);
        let cell = cell.as_ref();

        let mut guard1 = cell.borrow_mut().unwrap();
        let mut1 = &mut *guard1;
        assert_eq!(*mut1, VAL);
        *mut1 = VAL + 50;

        let no_alias_guard = cell.set_non_aliasing(mut1).unwrap();

        let mut guard2 = cell.borrow_mut().unwrap();
        let mut2 = &mut *guard2;
        assert_eq!(*mut2, VAL + 50);
        *mut2 = VAL - 30;
        drop(guard2);

        drop(no_alias_guard);

        assert_eq!(*mut1, VAL - 30);
        *mut1 = VAL - 5;

        drop(guard1);

        let guard3 = cell.borrow().unwrap();
        assert_eq!(*guard3, VAL - 5);
    }

    #[test]
    fn prevent_mut_mut_without_non_aliasing() {
        const VAL: i32 = 23456;
        let cell = GdCell::new(VAL);
        let cell = cell.as_ref();

        let mut guard1 = cell.borrow_mut().unwrap();
        let mut1 = &mut *guard1;
        assert_eq!(*mut1, VAL);
        *mut1 = VAL + 50;

        // let no_alias_guard = cell.set_non_aliasing(mut1).unwrap();

        cell.borrow_mut()
            .expect_err("reference may be aliasing so should be prevented");

        drop(guard1);
    }

    #[test]
    fn different_non_aliasing() {
        const VAL1: i32 = 23456;
        const VAL2: i32 = 11111;
        let cell1 = GdCell::new(VAL1);
        let cell1 = cell1.as_ref();
        let cell2 = GdCell::new(VAL2);
        let cell2 = cell2.as_ref();

        let mut guard1 = cell1.borrow_mut().unwrap();
        let mut1 = &mut *guard1;

        assert_eq!(*mut1, VAL1);
        *mut1 = VAL1 + 10;

        let mut guard2 = cell2.borrow_mut().unwrap();
        let mut2 = &mut *guard2;

        assert_eq!(*mut2, VAL2);
        *mut2 = VAL2 + 10;

        let no_alias_guard = cell1
            .set_non_aliasing(mut2)
            .expect_err("should not allow different references");

        drop(no_alias_guard);

        drop(guard1);
        drop(guard2);
    }
}
