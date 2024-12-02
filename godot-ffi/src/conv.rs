/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Conversions from some rust-types into appropriate Godot types.

use std::mem::size_of;

use crate as sys;
use crate::static_assert;

/// Infallibly convert `u32` into a `usize`.
///
/// gdext (and Godot in general) currently only supports targets where `u32` can be infallibly converted into a `usize`.
pub fn u32_to_usize(i: u32) -> usize {
    static_assert!(
        size_of::<u32>() <= size_of::<usize>(),
        "gdext only supports targets where u32 <= usize"
    );

    // SAFETY: The above static assert ensures that this can never fail.
    unsafe { i.try_into().unwrap_unchecked() }
}

/// Converts a Rust-bool into a sys-bool.
pub const fn bool_to_sys(value: bool) -> sys::GDExtensionBool {
    value as sys::GDExtensionBool
}

/// Converts a sys-bool to Rust-bool.
///
/// # Panics
/// If the value is not a valid sys-bool (0 or 1).
pub fn bool_from_sys(value: sys::GDExtensionBool) -> bool {
    match value {
        SYS_TRUE => true,
        SYS_FALSE => false,
        _ => panic!("Invalid GDExtensionBool value: {}", value),
    }
}

/// Convert a list into a pointer + length pair. Should be used together with [`ptr_list_from_sys`].
///
/// If `list_from_sys` is not called on this list then that will cause a memory leak.
#[cfg(since_api = "4.3")]
pub fn ptr_list_into_sys<T>(list: Vec<T>) -> (*const T, u32) {
    let len: u32 = list
        .len()
        .try_into()
        .expect("list must have length that fits in u32");
    let ptr = Box::leak(list.into_boxed_slice()).as_ptr();

    (ptr, len)
}

/// Get a list back from a previous call to [`ptr_list_into_sys`].
///
/// # Safety
/// - `ptr` must have been returned from a call to `list_into_sys`.
/// - `ptr` must be passed to this function exactly once and not used in any other context.
#[cfg(since_api = "4.3")]
#[deny(unsafe_op_in_unsafe_fn)]
pub unsafe fn ptr_list_from_sys<T>(ptr: *const T, len: u32) -> Box<[T]> {
    let ptr: *mut T = ptr.cast_mut();
    let len: usize = sys::conv::u32_to_usize(len);

    // SAFETY: `ptr` was created in `list_into_sys` from a slice of length `len`.
    // And has not been mutated since. It was mutable in the first place, but GDExtension API requires const ptr.
    let slice = unsafe { std::slice::from_raw_parts_mut(ptr, len) };

    // SAFETY: This is the first call to this function, and the list will not be accessed again after this function call.
    unsafe { Box::from_raw(slice) }
}

pub const SYS_TRUE: sys::GDExtensionBool = bool_to_sys(true);
pub const SYS_FALSE: sys::GDExtensionBool = bool_to_sys(false);

#[cfg(test)]
mod test {
    use crate::conv::{bool_to_sys, u32_to_usize, SYS_FALSE, SYS_TRUE};

    #[test]
    fn sys_bool() {
        assert_eq!(bool_to_sys(true), SYS_TRUE);
        assert_eq!(bool_to_sys(false), SYS_FALSE);
    }

    #[test]
    fn u32_into_usize_test() {
        const CHECKS: &[u32] = &[
            0,
            123,
            4444,
            u32::MAX,
            u16::MAX as u32,
            u8::MAX as u32,
            i8::MAX as u32,
            i16::MAX as u32,
            i32::MAX as u32,
        ];

        for value in CHECKS {
            assert_eq!(u32_to_usize(*value), *value as usize);
            assert_eq!(u32_to_usize(*value) as u32, *value);
        }
    }
}
