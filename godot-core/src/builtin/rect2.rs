/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot_ffi as sys;
use sys::{ffi_methods, GodotFfi};

use super::{real, Rect2i, Vector2};

/// 2D axis-aligned bounding box.
///
/// `Rect2` consists of a position, a size, and several utility functions. It is typically used for
/// fast overlap tests.
///
/// Currently most methods are only available through [`InnerRect2`](super::inner::InnerRect2).
///
/// The 3D counterpart to `Rect2` is [`Aabb`](super::Aabb).
#[derive(Default, Copy, Clone, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(C)]
pub struct Rect2 {
    pub position: Vector2,
    pub size: Vector2,
}

impl Rect2 {
    /// Create a new `Rect2` from a position and a size.
    ///
    /// _Godot equivalent: `Rect2(Vector2 position, Vector2 size)`_
    #[inline]
    pub const fn new(position: Vector2, size: Vector2) -> Self {
        Self { position, size }
    }

    /// Create a new `Rect2` from four reals representing position `(x,y)` and size `(width,height)`.
    ///
    /// _Godot equivalent: `Rect2(float x, float y, float width, float height)`_
    #[inline]
    pub const fn from_components(x: real, y: real, width: real, height: real) -> Self {
        Self {
            position: Vector2::new(x, y),
            size: Vector2::new(width, height),
        }
    }

    /// Create a new `Rect2` from a `Rect2i`, using `as` for `i32` to `real` conversions.
    ///
    /// _Godot equivalent: `Rect2(Rect2i from)`_
    #[inline]
    pub const fn from_rect2i(rect: Rect2i) -> Self {
        Self {
            position: Vector2::from_vector2i(rect.position),
            size: Vector2::from_vector2i(rect.size),
        }
    }

    /// Create a new `Rect2` with the first corner at `position` and the opposite corner at `end`.
    #[inline]
    pub fn from_corners(position: Vector2, end: Vector2) -> Self {
        Self {
            position,
            size: position + end,
        }
    }

    /// The end of the `Rect2` calculated as `position + size`.
    ///
    /// _Godot equivalent: `Rect2.size` property_
    #[inline]
    pub fn end(&self) -> Vector2 {
        self.position + self.size
    }

    /// Set size based on desired end-point.
    ///
    /// _Godot equivalent: `Rect2.size` property_
    #[inline]
    pub fn set_end(&mut self, end: Vector2) {
        self.size = end - self.position
    }

    /// Returns `true` if the two `Rect2`s are approximately equal, by calling `is_equal_approx` on
    /// `position` and `size`.
    ///
    /// _Godot equivalent: `Rect2.is_equal_approx()`_
    #[inline]
    pub fn is_equal_approx(&self, other: &Self) -> bool {
        self.position.is_equal_approx(other.position) && self.size.is_equal_approx(other.size)
    }

    /* Add in when `Rect2::abs()` is implemented.
    /// Assert that the size of the `Rect2` is not negative.
    ///
    /// Certain functions will fail to give a correct result if the size is negative.
    #[inline]
    pub fn assert_nonnegative(&self) {
        assert!(
            self.size.x >= 0.0 && self.size.y >= 0.0,
            "size {:?} is negative",
            self.size
        );
    }
    */
}

// SAFETY:
// This type is represented as `Self` in Godot, so `*mut Self` is sound.
unsafe impl GodotFfi for Rect2 {
    ffi_methods! { type sys::GDExtensionTypePtr = *mut Self; .. }
}

impl std::fmt::Display for Rect2 {
    /// Formats `Rect2` to match Godot's string representation.
    ///
    /// Example:
    /// ```
    /// use godot::prelude::*;
    /// let rect = Rect2::new(Vector2::new(0.0, 0.0), Vector2::new(1.0, 1.0));
    /// assert_eq!(format!("{}", rect), "[P: (0, 0), S: (1, 1)]");
    /// ```
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // godot output be like:
        // [P: (0, 0), S: (0, 0)]
        write!(f, "[P: {}, S: {}]", self.position, self.size)
    }
}

#[cfg(test)]
mod test {
    #[cfg(feature = "serde")]
    #[test]
    fn serde_roundtrip() {
        let rect = super::Rect2::default();
        let expected_json = "{\"position\":{\"x\":0.0,\"y\":0.0},\"size\":{\"x\":0.0,\"y\":0.0}}";

        crate::builtin::test_utils::roundtrip(&rect, expected_json);
    }
}
