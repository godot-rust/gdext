/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot_ffi as sys;
use sys::{ffi_methods, GodotFfi};

use super::Vector3;

/// Axis-aligned bounding box in 3D space.
///
/// `Aabb` consists of a position, a size, and several utility functions. It is typically used for
/// fast overlap tests.
///
/// Currently most methods are only available through [`InnerAabb`](super::inner::InnerAabb).
///
/// The 2D counterpart to `Aabb` is [`Rect2`](super::Rect2).
#[derive(Default, Copy, Clone, PartialEq, Debug)]
#[repr(C)]
pub struct Aabb {
    pub position: Vector3,
    pub size: Vector3,
}

impl Aabb {
    /// Create a new `Aabb` from a position and a size.
    ///
    /// _Godot equivalent: `Aabb(Vector3 position, Vector3 size)`_
    #[inline]
    pub const fn new(position: Vector3, size: Vector3) -> Self {
        Self { position, size }
    }

    /// Create a new `Aabb` with the first corner at `position` and opposite corner at `end`.
    #[inline]
    pub fn from_corners(position: Vector3, end: Vector3) -> Self {
        Self {
            position,
            size: position + end,
        }
    }

    /// The end of the `Aabb` calculated as `position + size`.
    ///
    /// _Godot equivalent: `Aabb.size` property_
    #[inline]
    pub fn end(&self) -> Vector3 {
        self.position + self.size
    }

    /// Set size based on desired end-point.
    ///
    /// _Godot equivalent: `Aabb.size` property_
    #[inline]
    pub fn set_end(&mut self, end: Vector3) {
        self.size = end - self.position
    }

    /// Returns `true` if the two `Aabb`s are approximately equal, by calling `is_equal_approx` on
    /// `position` and `size`.
    ///
    /// _Godot equivalent: `Aabb.is_equal_approx()`_
    #[inline]
    pub fn is_equal_approx(&self, other: &Self) -> bool {
        self.position.is_equal_approx(other.position) && self.size.is_equal_approx(other.size)
    }

    /* Add in when `Aabb::abs()` is implemented.
    /// Assert that the size of the `Aabb` is not negative.
    ///
    /// Certain functions will fail to give a correct result if the size is negative.
    #[inline]
    pub fn assert_nonnegative(&self) {
        assert!(
            self.size.x >= 0.0 && self.size.y >= 0.0 && self.size.z >= 0.0,
            "size {:?} is negative",
            self.size
        );
    }
    */
}

impl GodotFfi for Aabb {
    ffi_methods! { type sys::GDExtensionTypePtr = *mut Self; .. }
}
