/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot_ffi as sys;
use sys::{ffi_methods, GodotFfi};

use super::{Rect2, Vector2i};

/// 2D axis-aligned integer bounding box.
///
/// `Rect2i` consists of a position, a size, and several utility functions. It is typically used for
/// fast overlap tests.
///
/// Currently most methods are only available through [`InnerRect2i`](super::inner::InnerRect2i).
#[derive(Default, Copy, Clone, Eq, PartialEq, Debug)]
#[repr(C)]
pub struct Rect2i {
    pub position: Vector2i,
    pub size: Vector2i,
}

impl Rect2i {
    /// Create a new `Rect2i` from a position and a size.
    ///
    /// _Godot equivalent: `Rect2i(Vector2i position, Vector2i size)`_
    #[inline]
    pub const fn new(position: Vector2i, size: Vector2i) -> Self {
        Self { position, size }
    }

    /// Create a new `Rect2i` from four `i32`s representing position `(x,y)` and size `(width,height)`.
    ///
    /// _Godot equivalent: `Rect2i(float x, float y, float width, float height)`_
    #[inline]
    pub const fn from_components(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            position: Vector2i::new(x, y),
            size: Vector2i::new(width, height),
        }
    }

    /// Create a new `Rect2i` from a `Rect2`, using `as` for `real` to `i32` conversions.
    ///
    /// _Godot equivalent: `Rect2i(Rect2 from)`_
    #[inline]
    pub const fn from_rect2(rect: Rect2) -> Self {
        Self {
            position: Vector2i::from_vector2(rect.position),
            size: Vector2i::from_vector2(rect.size),
        }
    }

    /// Create a new `Rect2i` with the first corner at `position` and the opposite corner at `end`.
    #[inline]
    pub fn from_corners(position: Vector2i, end: Vector2i) -> Self {
        Self {
            position,
            size: position + end,
        }
    }

    /// The end of the `Rect2i` calculated as `position + size`.
    ///
    /// _Godot equivalent: `Rect2i.size` property_
    #[inline]
    pub const fn end(&self) -> Vector2i {
        Vector2i::new(self.position.x + self.size.x, self.position.y + self.size.y)
    }

    /// Set size based on desired end-point.
    ///
    /// _Godot equivalent: `Rect2i.size` property_
    #[inline]
    pub fn set_end(&mut self, end: Vector2i) {
        self.size = end - self.position
    }

    /* Add in when `Rect2i::abs()` is implemented.
    /// Assert that the size of the `Rect2i` is not negative.
    ///
    /// Certain functions will fail to give a correct result if the size is negative.
    #[inline]
    pub const fn assert_nonnegative(&self) {
        assert!(
            self.size.x >= 0.0 && self.size.y >= 0.0,
            "size {:?} is negative",
            self.size
        );
    }
    */
}

impl GodotFfi for Rect2i {
    ffi_methods! { type sys::GDExtensionTypePtr = *mut Self; .. }
}
