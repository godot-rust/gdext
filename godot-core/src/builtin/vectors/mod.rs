/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// Must be first.
mod vector_macros;

mod vector2;
mod vector2i;
mod vector3;
mod vector3i;
mod vector4;
mod vector4i;
mod vector_axis;
mod vector_swizzle;

pub use vector2::*;
pub use vector2i::*;
pub use vector3::*;
pub use vector3i::*;
pub use vector4::*;
pub use vector4i::*;
pub use vector_axis::*;
pub use vector_swizzle::*;

pub use crate::swizzle;
