/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot_ffi::out;

use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

use crate::storage::{MutGuard, RefGuard};

use super::{Gd, GodotClass};

/// Immutably/shared bound reference guard for a [`Gd`][crate::obj::Gd] smart pointer.
///
/// See [`Gd::bind`][crate::obj::Gd::bind] for usage.
#[derive(Debug)]
pub struct GdRef<'a, T: GodotClass> {
    guard: RefGuard<'a, T>,
}

impl<'a, T: GodotClass> GdRef<'a, T> {
    pub(crate) fn from_guard(guard: RefGuard<'a, T>) -> Self {
        Self { guard }
    }
}

impl<T: GodotClass> Deref for GdRef<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.guard
    }
}

impl<T: GodotClass> Drop for GdRef<'_, T> {
    fn drop(&mut self) {
        out!("GdRef drop: {:?}", std::any::type_name::<T>());
    }
}

// TODO Clone or Share

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Mutably/exclusively bound reference guard for a [`Gd`][crate::obj::Gd] smart pointer.
///
/// See [`Gd::bind_mut`][crate::obj::Gd::bind_mut] for usage.
#[derive(Debug)]
pub struct GdMut<'a, T: GodotClass> {
    guard: MutGuard<'a, T>,
}

impl<'a, T: GodotClass> GdMut<'a, T> {
    pub(crate) fn from_guard(guard: MutGuard<'a, T>) -> Self {
        Self { guard }
    }
}

impl<T: GodotClass> Deref for GdMut<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.guard
    }
}

impl<T: GodotClass> DerefMut for GdMut<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.guard
    }
}

impl<T: GodotClass> Drop for GdMut<'_, T> {
    fn drop(&mut self) {
        out!("GdMut drop: {:?}", std::any::type_name::<T>());
    }
}

pub struct BaseRef<'a, T: GodotClass> {
    pub(crate) gd: Gd<T>,
    pub(crate) _p: PhantomData<&'a ()>,
}

impl<T: GodotClass> Deref for BaseRef<'_, T> {
    type Target = Gd<T>;

    fn deref(&self) -> &Gd<T> {
        &self.gd
    }
}

pub struct BaseMut<'a, T: GodotClass> {
    pub(crate) gd: Gd<T>,
    pub(crate) _p: PhantomData<&'a mut ()>,
}

impl<T: GodotClass> Deref for BaseMut<'_, T> {
    type Target = Gd<T>;

    fn deref(&self) -> &Gd<T> {
        &self.gd
    }
}

impl<T: GodotClass> DerefMut for BaseMut<'_, T> {
    fn deref_mut(&mut self) -> &mut Gd<T> {
        &mut self.gd
    }
}
