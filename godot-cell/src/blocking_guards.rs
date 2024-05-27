/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Condvar, Mutex};

use crate::blocking_cell::ThreadTracker;
use crate::guards::{MutGuard, RefGuard};

/// Extended version of [`panicking::RefGuard`](crate::panicking::RefGuard) that tracks which thread a reference belongs to and when it's dropped.
///
/// See [`panicking::RefGuard`](crate::panicking::RefGuard) for more details.
#[derive(Debug)]
pub struct RefGuardBlocking<'a, T> {
    inner: Option<RefGuard<'a, T>>,
    mut_condition: Arc<Condvar>,
    state: Arc<Mutex<ThreadTracker>>,
}

impl<'a, T> RefGuardBlocking<'a, T> {
    pub(crate) fn new(
        inner: RefGuard<'a, T>,
        mut_condition: Arc<Condvar>,
        state: Arc<Mutex<ThreadTracker>>,
    ) -> Self {
        Self {
            inner: Some(inner),
            mut_condition,
            state,
        }
    }
}

impl<'a, T> Deref for RefGuardBlocking<'a, T> {
    type Target = <RefGuard<'a, T> as Deref>::Target;

    fn deref(&self) -> &Self::Target {
        self.inner.as_ref().unwrap().deref()
    }
}

impl<'a, T> Drop for RefGuardBlocking<'a, T> {
    fn drop(&mut self) {
        let mut state_lock = self.state.lock().unwrap();

        state_lock.decrement_current_thread_shared_count();

        drop(self.inner.take());

        self.mut_condition.notify_one();
        drop(state_lock);
    }
}

/// Extended version of [`panicking::MutGuard`](crate::panicking::MutGuard) that tracks which thread a reference belongs to and when it's dropped.
///
/// See [`panicking::MutGuard`](crate::panicking::MutGuard) for more details.
#[derive(Debug)]
pub struct MutGuardBlocking<'a, T> {
    inner: Option<MutGuard<'a, T>>,
    mut_condition: Arc<Condvar>,
    immut_condition: Arc<Condvar>,
}

impl<'a, T> MutGuardBlocking<'a, T> {
    pub(crate) fn new(
        inner: MutGuard<'a, T>,
        mut_condition: Arc<Condvar>,
        immut_condition: Arc<Condvar>,
    ) -> Self {
        Self {
            inner: Some(inner),
            immut_condition,
            mut_condition,
        }
    }
}

impl<'a, T> Deref for MutGuardBlocking<'a, T> {
    type Target = <MutGuard<'a, T> as Deref>::Target;

    fn deref(&self) -> &Self::Target {
        self.inner.as_ref().unwrap().deref()
    }
}

impl<'a, T> DerefMut for MutGuardBlocking<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner.as_mut().unwrap().deref_mut()
    }
}

impl<'a, T> Drop for MutGuardBlocking<'a, T> {
    fn drop(&mut self) {
        drop(self.inner.take());

        self.mut_condition.notify_one();
        self.immut_condition.notify_all();
    }
}
