/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::mem::MaybeUninit;
use std::ptr;

use crate::sys;

/// A fixed-size buffer that does not do any allocations, and can hold up to `N` elements of type `T`.
///
/// This is used to implement [`PackedArray::extend()`][crate::builtin::PackedArray::extend] in an efficient way, because it forms a middle
/// ground between repeated `push()` calls (slow) and first collecting the entire `Iterator` into a `Vec` (faster, but takes more memory).
///
/// Note that `N` must not be 0 for the buffer to be useful.
///
/// The public API is implemented via the trait [`ExtendBufferTrait<T>`]. This is a necessity for generic programming, since Rust does not
/// permit using the const-generic `N` in generic functions.
#[doc(hidden)] // Public; used in associated type [`PackedArrayElement::ExtendBuffer`][crate::meta::PackedArrayElement::ExtendBuffer].
pub struct ExtendBuffer<T, const N: usize> {
    buf: [MaybeUninit<T>; N],
    len: usize,
}

impl<T, const N: usize> Default for ExtendBuffer<T, N> {
    fn default() -> Self {
        Self {
            buf: [const { MaybeUninit::uninit() }; N],
            len: 0,
        }
    }
}

impl<T, const N: usize> ExtendBufferTrait<T> for ExtendBuffer<T, N> {
    /// Appends the given value to the buffer.
    ///
    /// # Panics
    /// If the buffer is full.
    fn push(&mut self, value: T) {
        self.buf[self.len].write(value);
        self.len += 1;
    }

    /// Returns `true` iff the buffer is full.
    fn is_full(&self) -> bool {
        self.len == N
    }

    /// Returns a slice of all initialized elements in the buffer, and sets the length of the buffer back to 0.
    ///
    /// It is the caller's responsibility to ensure that all elements in the returned slice get dropped!
    fn drain_as_mut_slice(&mut self) -> &mut [T] {
        // Prevent panic in self.buf[0] below.
        if N == 0 {
            return &mut [];
        }
        sys::strict_assert!(self.len <= N);

        let len = self.len;
        self.len = 0;

        // MaybeUninit::slice_assume_init_ref() could be used here instead, but it's experimental.
        //
        // SAFETY:
        // - The pointer is non-null, valid and aligned.
        // - `len` elements are always initialized.
        // - The memory is not accessed through any other pointer, because we hold a `&mut` reference to `self`.
        // - `len * mem::size_of::<T>()` is no larger than `isize::MAX`, otherwise the `buf` slice could not have existed either.
        unsafe { std::slice::from_raw_parts_mut(self.buf[0].as_mut_ptr(), len) }
    }
}

impl<T, const N: usize> Drop for ExtendBuffer<T, N> {
    fn drop(&mut self) {
        // Prevent panic in self.buf[0] below.
        if N == 0 {
            return;
        }
        sys::strict_assert!(self.len <= N);

        // SAFETY: `slice_from_raw_parts_mut` by itself is not unsafe, but to make the resulting slice safe to use:
        // - `self.buf[0]` is a valid pointer, exactly `self.len` elements are initialized.
        // - The pointer is not aliased since we have an exclusive `&mut self`.
        let slice = ptr::slice_from_raw_parts_mut(self.buf[0].as_mut_ptr(), self.len);

        // SAFETY: the value is valid because the `slice_from_raw_parts_mut` requirements are met,
        // and there is no other way to access the value.
        unsafe {
            ptr::drop_in_place(slice);
        }
    }
}

#[test]
fn test_extend_buffer_drop() {
    // We use an `Rc` to test the buffer's `drop` behavior.
    use std::rc::Rc;

    let mut buf = ExtendBuffer::<Rc<i32>, 1>::default();
    let value = Rc::new(42);
    buf.push(Rc::clone(&value));

    // The buffer contains one strong reference, this function contains another.
    assert_eq!(Rc::strong_count(&value), 2);

    let slice = buf.drain_as_mut_slice();

    // The strong reference has been returned in the slice, but not dropped.
    assert_eq!(Rc::strong_count(&value), 2);

    // SAFETY:
    // - The slice returned by `drain_as_mut_slice` is valid, and therefore so is its first element.
    // - There is no way to access parts of `slice[0]` while `drop_in_place` is executing.
    unsafe {
        ptr::drop_in_place(&mut slice[0]);
    }

    // The reference held by the slice has now been dropped.
    assert_eq!(Rc::strong_count(&value), 1);

    drop(buf);

    // The buffer has not dropped another reference.
    assert_eq!(Rc::strong_count(&value), 1);
}

/// Trait abstracting ExtendBuffer operations for different buffer sizes.
///
/// This is necessary because Rust can currently not use `N` const-generic in a generic function:
/// `error[E0401]: can't use generic parameters from outer item`
#[doc(hidden)] // Public; used in associated type [`PackedArrayElement::ExtendBuffer`][crate::meta::PackedArrayElement::ExtendBuffer].
pub trait ExtendBufferTrait<T> {
    fn push(&mut self, value: T);
    fn is_full(&self) -> bool;
    fn drain_as_mut_slice(&mut self) -> &mut [T];
}
