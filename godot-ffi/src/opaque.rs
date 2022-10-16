/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// Note: transmute not supported for const generics; see
// https://users.rust-lang.org/t/transmute-in-the-context-of-constant-generics/56827

/// Stores an opaque object of a certain size, with very restricted operations
///
/// Note: due to `align(8)` and not `packed` repr, this type may be bigger than `N` bytes
/// (which should be OK since C++ just needs to read/write those `N` bytes reliably).
#[repr(C, align(8))]
#[derive(Copy, Clone)]
pub struct Opaque<const N: usize> {
    storage: [u8; N],
    marker: std::marker::PhantomData<*const u8>, // disable Send/Sync
}
