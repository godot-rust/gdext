/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::marker::PhantomData;
use std::mem::ManuallyDrop;

use godot_ffi::GodotNullableFfi;

use crate::obj::{GodotClass, RawGd};

/// Represents a successful low-level cast from `T` to `U`.
///
/// This exists to provide a safe API for casting, without the need for clone (and thus ref-count increments).
///
/// It achieves this by keeping the destination (cast result) as a weak pointer. If dropped, nothing happens.
/// To extract the destination, the caller must submit a strong pointer of the source type `T` in exchange.
///
/// See [`RawGd::ffi_cast()`].
pub(crate) struct CastSuccess<T: GodotClass, U: GodotClass> {
    _phantom: PhantomData<*mut T>,

    /// Weak pointer. Drop does not decrement ref-count.
    dest: ManuallyDrop<RawGd<U>>,
}

impl<T: GodotClass, U: GodotClass> CastSuccess<T, U> {
    /// Create from weak pointer.
    pub(crate) fn from_weak(weak: RawGd<U>) -> CastSuccess<T, U>
    where
        U: GodotClass,
    {
        Self {
            _phantom: PhantomData,
            dest: ManuallyDrop::new(weak),
        }
    }

    /// Successful cast from null to null.
    pub fn null() -> Self {
        Self {
            _phantom: PhantomData,
            dest: ManuallyDrop::new(RawGd::null()),
        }
    }

    /// Access shared reference to destination, without consuming object.
    #[cfg(debug_assertions)]
    pub fn as_dest_ref(&self) -> &RawGd<U> {
        self.check_validity();
        &self.dest
    }

    /// Access exclusive reference to destination, without consuming object.
    pub fn as_dest_mut(&mut self) -> &mut RawGd<U> {
        self.check_validity();
        &mut self.dest
    }

    /// Extracts destination object, sacrificing the source in exchange.
    ///
    /// This trade is needed because the result is a weak pointer (no ref-count increment). By submitting a strong pointer in its place,
    /// we can retain the overall ref-count balance.
    pub fn into_dest(self, traded_source: RawGd<T>) -> RawGd<U> {
        debug_assert_eq!(
            traded_source.instance_id_unchecked(),
            self.dest.instance_id_unchecked(),
            "traded_source must point to the same object as the destination"
        );
        self.check_validity();

        std::mem::forget(traded_source);
        ManuallyDrop::into_inner(self.dest)
    }

    fn check_validity(&self) {
        debug_assert!(self.dest.is_null() || self.dest.is_instance_valid());
    }
}
