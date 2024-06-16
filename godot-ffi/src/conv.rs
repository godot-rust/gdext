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

/// Converts a rust-bool into a sys-bool.
pub const fn bool_to_sys(value: bool) -> sys::GDExtensionBool {
    value as sys::GDExtensionBool
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
