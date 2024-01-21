/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// Note: transmute not supported for const generics; see
// https://users.rust-lang.org/t/transmute-in-the-context-of-constant-generics/56827

/// Stores an opaque object of a certain size, with very restricted operations
///
/// Note: due to `align(4)` / `align(8)` and not `packed` repr, this type may be bigger than `N` bytes
/// (which should be OK since C++ just needs to read/write those `N` bytes reliably).
///
///
/// For float/double inference, see:
/// * https://github.com/godotengine/godot-proposals/issues/892
/// * https://github.com/godotengine/godot-cpp/pull/728
///
/// We have to do a `target_pointer_width` check *after* code generation, see https://github.com/rust-lang/rust/issues/42587.
#[cfg_attr(target_pointer_width = "32", repr(C, align(4)))]
#[cfg_attr(target_pointer_width = "64", repr(C, align(8)))]
#[derive(Copy, Clone)]
pub struct Opaque<const N: usize> {
    storage: [u8; N],
    marker: std::marker::PhantomData<*const u8>, // disable Send/Sync
}
