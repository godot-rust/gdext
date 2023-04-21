/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
#![macro_use]

use crate::builtin::real;
use crate::builtin::{Vector2, Vector2i, Vector3, Vector3i, Vector4, Vector4i};
use godot_ffi as sys;
use sys::{ffi_methods, GodotFfi};

macro_rules! swizzle {
    ($vec:expr => $a:ident, $b:ident) => {{
        let expr = $vec;
        (expr.$a, expr.$b).into()
    }};
    ($vec:expr => $a:ident, $b:ident, $c:ident) => {{
        let expr = $vec;
        (expr.$a, expr.$b, expr.$c).into()
    }};
    ($vec:expr => $a:ident, $b:ident, $c:ident, $d:ident) => {{
        let expr = $vec;
        (expr.$a, expr.$b, expr.$c, expr.$d).into()
    }};
}

/// Enumerates the axes in a [`Vector2`].
#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(i32)]
pub enum Vector2Axis {
    /// The X axis.
    X,
    /// The Y axis.
    Y,
}

// SAFETY:
// This type is represented as `Self` in Godot, so `*mut Self` is sound.
unsafe impl GodotFfi for Vector2Axis {
    ffi_methods! { type sys::GDExtensionTypePtr = *mut Self; .. }
}

/// Enumerates the axes in a [`Vector3`].
// TODO auto-generate this, alongside all the other builtin type's enums
#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(i32)]
pub enum Vector3Axis {
    /// The X axis.
    X,
    /// The Y axis.
    Y,
    /// The Z axis.
    Z,
}

// SAFETY:
// This type is represented as `Self` in Godot, so `*mut Self` is sound.
unsafe impl GodotFfi for Vector3Axis {
    ffi_methods! { type sys::GDExtensionTypePtr = *mut Self; .. }
}

/// Enumerates the axes in a [`Vector4`].
#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(i32)]
pub enum Vector4Axis {
    /// The X axis.
    X,
    /// The Y axis.
    Y,
    /// The Z axis.
    Z,
    /// The W axis.
    W,
}

// SAFETY:
// This type is represented as `Self` in Godot, so `*mut Self` is sound.
unsafe impl GodotFfi for Vector4Axis {
    ffi_methods! { type sys::GDExtensionTypePtr = *mut Self; .. }
}

impl_vector_index!(Vector2, real, (x, y), Vector2Axis, (X, Y));
impl_vector_index!(Vector2i, i32, (x, y), Vector2Axis, (X, Y));

impl_vector_index!(Vector3, real, (x, y, z), Vector3Axis, (X, Y, Z));
impl_vector_index!(Vector3i, i32, (x, y, z), Vector3Axis, (X, Y, Z));

impl_vector_index!(Vector4, real, (x, y, z, w), Vector4Axis, (X, Y, Z, W));
impl_vector_index!(Vector4i, i32, (x, y, z, w), Vector4Axis, (X, Y, Z, W));
