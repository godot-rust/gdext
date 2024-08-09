/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::num::NonZeroU64;

use godot_ffi as sys;
use sys::{ffi_methods, static_assert, static_assert_eq_size_align, GodotFfi};


pub trait Rid {
    to
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct PhysicsRid {
    id: NonZeroU64
}


// TextRid