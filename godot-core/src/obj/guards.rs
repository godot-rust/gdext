/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot_ffi::out;

#[cfg(not(feature = "experimental-threads"))]
use std::cell;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
#[cfg(feature = "experimental-threads")]
use std::sync;

use super::{Gd, GodotClass};

/// Immutably/shared bound reference guard for a [`Gd`][crate::obj::Gd] smart pointer.
///
/// See [`Gd::bind`][crate::obj::Gd::bind] for usage.
#[derive(Debug)]
pub struct GdRef<'a, T> {
    #[cfg(not(feature = "experimental-threads"))]
    cell_ref: cell::Ref<'a, T>,

    #[cfg(feature = "experimental-threads")]
    cell_ref: sync::RwLockReadGuard<'a, T>,
}

impl<'a, T> GdRef<'a, T> {
    #[cfg(not(feature = "experimental-threads"))]
    pub(crate) fn from_cell(cell_ref: cell::Ref<'a, T>) -> Self {
        Self { cell_ref }
    }

    #[cfg(feature = "experimental-threads")]
    pub(crate) fn from_cell(cell_ref: sync::RwLockReadGuard<'a, T>) -> Self {
        out!("GdRef init: {:?}", std::any::type_name::<T>());
        Self { cell_ref }
    }
}

impl<T> Deref for GdRef<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.cell_ref.deref()
    }
}

impl<T> Drop for GdRef<'_, T> {
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
pub struct GdMut<'a, T> {
    #[cfg(not(feature = "experimental-threads"))]
    cell_ref: cell::RefMut<'a, T>,

    #[cfg(feature = "experimental-threads")]
    cell_ref: sync::RwLockWriteGuard<'a, T>,
}

impl<'a, T> GdMut<'a, T> {
    #[cfg(not(feature = "experimental-threads"))]
    pub(crate) fn from_cell(cell_ref: cell::RefMut<'a, T>) -> Self {
        Self { cell_ref }
    }

    #[cfg(feature = "experimental-threads")]
    pub(crate) fn from_cell(cell_ref: sync::RwLockWriteGuard<'a, T>) -> Self {
        out!("GdMut init: {:?}", std::any::type_name::<T>());
        Self { cell_ref }
    }
}

impl<T> Deref for GdMut<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.cell_ref.deref()
    }
}

impl<T> DerefMut for GdMut<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.cell_ref.deref_mut()
    }
}

impl<T> Drop for GdMut<'_, T> {
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
