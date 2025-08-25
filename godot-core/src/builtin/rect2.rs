/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot_ffi as sys;
use sys::{ffi_methods, ExtVariantType, GodotFfi};

use crate::builtin::math::ApproxEq;
use crate::builtin::{real, Rect2i, Side, Vector2};

/// 2D axis-aligned bounding box.
///
/// `Rect2` consists of a position, a size, and several utility functions. It is typically used for
/// fast overlap tests.
///
/// # All bounding-box types
///
/// | Dimension | Floating-point  | Integer      |
/// |-----------|-----------------|--------------|
/// | 2D        | **`Rect2`**     | [`Rect2i`]   |
/// | 3D        | [`Aabb`]        |              |
///
/// <br>You can convert to `Rect2i` using [`cast_int()`][Self::cast_int].
///
/// [`Aabb`]: crate::builtin::Aabb
///
/// # Godot docs
///
/// [`Rect2` (stable)](https://docs.godotengine.org/en/stable/classes/class_rect2.html)
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

    /// Create a new `Rect2` with the first corner at `position` and the opposite corner at `end`.
    #[inline]
    pub fn from_corners(position: Vector2, end: Vector2) -> Self {
        // Cannot use floating point arithmetic in const functions.
        Self::new(position, end - position)
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

    /// Create a new `Rect2i` from a `Rect2`, using `as` for `real` to `i32` conversions.
    ///
    /// _Godot equivalent: `Rect2i(Rect2 from)`_
    #[inline]
    pub const fn cast_int(self) -> Rect2i {
        Rect2i {
            position: self.position.cast_int(),
            size: self.size.cast_int(),
        }
    }

    /// Returns a rectangle with the same geometry, with top-left corner as `position` and non-negative size.
    #[inline]
    pub fn abs(self) -> Self {
        Self {
            position: self.position + self.size.coord_min(Vector2::ZERO),
            size: self.size.abs(),
        }
    }

    /// Whether `self` covers at least the entire area of `b` (and possibly more).
    #[inline]
    pub fn encloses(self, b: Rect2) -> bool {
        let end = self.end();
        let b_end = b.end();

        b.position.x >= self.position.x
            && b.position.y >= self.position.y
            && b_end.x <= end.x
            && b_end.y <= end.y
    }

    /// Returns a copy of this rectangle expanded to include a given point.
    ///
    /// Note: This method is not reliable for `Rect2` with a negative size. Use [`abs`][Self::abs]
    /// to get a positive sized equivalent rectangle for expanding.
    #[inline]
    pub fn expand(self, to: Vector2) -> Self {
        self.merge(Rect2::new(to, Vector2::ZERO))
    }

    /// Returns a larger rectangle that contains this `Rect2` and `b`.
    ///
    /// Note: This method is not reliable for `Rect2` with a negative size. Use [`abs`][Self::abs]
    /// to get a positive sized equivalent rectangle for merging.
    #[inline]
    pub fn merge(self, b: Self) -> Self {
        let position = self.position.coord_min(b.position);
        let end = self.end().coord_max(b.end());

        Self::from_corners(position, end)
    }

    /// Returns the area of the rectangle.
    #[inline]
    pub fn area(self) -> real {
        self.size.x * self.size.y
    }

    /// Returns the center of the Rect2, which is equal to `position + (size / 2)`.
    #[inline]
    pub fn center(self) -> Vector2 {
        self.position + (self.size / 2.0)
    }

    /// Returns a copy of the Rect2 grown by the specified `amount` on all sides.
    #[inline]
    #[must_use]
    pub fn grow(self, amount: real) -> Self {
        let position = self.position - Vector2::new(amount, amount);
        let size = self.size + Vector2::new(amount, amount) * 2.0;

        Self { position, size }
    }

    /// Returns a copy of the Rect2 grown by the specified amount on each side individually.
    #[inline]
    pub fn grow_individual(self, left: real, top: real, right: real, bottom: real) -> Self {
        Self::from_components(
            self.position.x - left,
            self.position.y - top,
            self.size.x + left + right,
            self.size.y + top + bottom,
        )
    }

    /// Returns a copy of the `Rect2` grown by the specified `amount` on the specified `RectSide`.
    ///
    /// `amount` may be negative, but care must be taken: If the resulting `size` has
    /// negative components the computation may be incorrect.
    #[inline]
    pub fn grow_side(self, side: Side, amount: real) -> Self {
        match side {
            Side::LEFT => self.grow_individual(amount, 0.0, 0.0, 0.0),
            Side::TOP => self.grow_individual(0.0, amount, 0.0, 0.0),
            Side::RIGHT => self.grow_individual(0.0, 0.0, amount, 0.0),
            Side::BOTTOM => self.grow_individual(0.0, 0.0, 0.0, amount),
        }
    }

    /// Returns `true` if the Rect2 has area, and `false` if the Rect2 is linear, empty, or has a negative size. See also `get_area`.
    #[inline]
    pub fn has_area(self) -> bool {
        self.size.x > 0.0 && self.size.y > 0.0
    }

    /// Returns `true` if the Rect2 contains a point (excluding right/bottom edges).
    ///
    /// By convention, the right and bottom edges of the Rect2 are considered exclusive, so points on these edges are not included.
    ///
    /// Note: This method is not reliable for Rect2 with a negative size. Use `abs` to get a positive sized equivalent rectangle to check for contained points.
    #[inline]
    #[doc(alias = "has_point")]
    pub fn contains_point(self, point: Vector2) -> bool {
        let point = point - self.position;

        point.abs() == point && point.x < self.size.x && point.y < self.size.y
    }

    /// Returns the intersection of this Rect2 and `b`. If the rectangles do not intersect, an empty Rect2 is returned.
    #[inline]
    pub fn intersect(self, b: Self) -> Option<Self> {
        if !self.intersects(b) {
            return None;
        }

        let mut rect = b;
        rect.position = rect.position.coord_max(self.position);

        let end = self.end();
        let end_b = b.end();
        rect.size = end.coord_min(end_b) - rect.position;

        Some(rect)
    }

    /// Checks whether two rectangles have at least one point in common.
    ///
    /// Also returns `true` if the rects only touch each other (share a point/edge).
    /// See [`intersects_exclude_borders`][Self::intersects_exclude_borders] if you want to return `false` in that case.
    ///
    /// _Godot equivalent: `Rect2.intersects(Rect2 b, bool include_borders = true)`_
    #[inline]
    pub fn intersects(self, b: Self) -> bool {
        let end = self.end();
        let end_b = b.end();

        self.position.x <= end_b.x
            && end.x >= b.position.x
            && self.position.y <= end_b.y
            && end.y >= b.position.y
    }

    /// Checks whether two rectangles have at least one _inner_ point in common (not on the borders).
    ///
    /// Returns `false` if the rects only touch each other (share a point/edge).
    /// See [`intersects`][Self::intersects] if you want to return `true` in that case.
    ///
    /// _Godot equivalent: `Rect2.intersects(AABB b, bool include_borders = false)`_
    #[inline]
    pub fn intersects_exclude_borders(self, b: Self) -> bool {
        let end = self.end();
        let end_b = b.end();

        self.position.x < end_b.x
            && end.x > b.position.x
            && self.position.y < end_b.y
            && end.y > b.position.y
    }

    /// Returns `true` if this Rect2 is finite, by calling `@GlobalScope.is_finite` on each component.
    #[inline]
    pub fn is_finite(self) -> bool {
        self.position.is_finite() && self.size.is_finite()
    }

    /// The end of the `Rect2` calculated as `position + size`.
    #[inline]
    pub fn end(self) -> Vector2 {
        self.position + self.size
    }

    /// Set size based on desired end-point.
    #[inline]
    pub fn set_end(&mut self, end: Vector2) {
        self.size = end - self.position
    }

    /// Assert that the size of the `Rect2` is not negative.
    ///
    /// Certain functions will fail to give a correct result if the size is negative.
    #[inline]
    pub fn assert_nonnegative(self) {
        assert!(
            self.size.x >= 0.0 && self.size.y >= 0.0,
            "size {:?} is negative",
            self.size
        );
    }
}

// SAFETY:
// This type is represented as `Self` in Godot, so `*mut Self` is sound.
unsafe impl GodotFfi for Rect2 {
    const VARIANT_TYPE: ExtVariantType = ExtVariantType::Concrete(sys::VariantType::RECT2);

    ffi_methods! { type sys::GDExtensionTypePtr = *mut Self; .. }
}

crate::meta::impl_godot_as_self!(Rect2: ByValue);

impl ApproxEq for Rect2 {
    /// Returns if the two `Rect2`s are approximately equal, by comparing `position` and `size` separately.
    #[inline]
    fn approx_eq(&self, other: &Self) -> bool {
        Vector2::approx_eq(&self.position, &other.position)
            && Vector2::approx_eq(&self.size, &other.size)
    }
}

impl std::fmt::Display for Rect2 {
    /// Formats `Rect2` to match Godot's string representation.
    ///
    /// # Example
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
