/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
use std::cmp;

use godot_ffi as sys;
use sys::{ffi_methods, GodotFfi};

use super::{Rect2, RectSide, Vector2i};

/// 2D axis-aligned integer bounding box.
///
/// `Rect2i` consists of a position, a size, and several utility functions. It is typically used for
/// fast overlap tests.
#[derive(Default, Copy, Clone, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(C)]
pub struct Rect2i {
    /// The position of the rectangle.
    pub position: Vector2i,

    /// The size of the rectangle.
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
            size: end - position,
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

    /// Returns a `Rect2i` with equivalent position and area, modified so that the top-left corner
    /// is the origin and `width` and `height` are positive.
    #[inline]
    pub fn abs(self) -> Self {
        let abs_size = self.size.abs();
        let offset = Vector2i::new(cmp::min(self.size.x, 0), cmp::min(self.size.y, 0));
        Self::new(self.position + offset, abs_size)
    }

    /// Returns `true` if this `Rect2i` completely encloses another one.
    ///
    /// Any `Rect2i` encloses itself, i.e. an enclosed `Rect2i` does is not required to be a
    /// proper sub-rect.
    #[inline]
    pub const fn encloses(&self, other: Self) -> bool {
        self.assert_nonnegative();
        other.assert_nonnegative();

        let own_end = self.end();
        let other_end = other.end();
        other.position.x >= self.position.x
            && other.position.y >= self.position.y
            && other_end.x <= own_end.x
            && other_end.y <= own_end.y
    }

    /// Returns a copy of this `Rect2i` expanded so that the borders align with the given point.
    #[inline]
    pub fn expand(self, to: Vector2i) -> Self {
        self.assert_nonnegative();

        let begin = self.position;
        let end = self.end();
        Self::from_corners(begin.coord_min(to), end.coord_max(to))
    }

    /// Returns the area of the `Rect2i`.
    ///
    /// _Godot equivalent: `Rect2i.get_area` function_
    #[doc(alias = "get_area")]
    #[inline]
    pub const fn area(&self) -> i32 {
        self.size.x * self.size.y
    }

    /// Returns the center of the `Rect2i`, which is equal to `position + (size / 2)`.
    ///
    /// If `size` is an odd number, the returned center value will be rounded towards `position`.
    ///
    /// _Godot equivalent: `Rect2i.get_center` function_
    #[doc(alias = "get_center")]
    #[inline]
    pub fn center(&self) -> Vector2i {
        self.position + (self.size / 2)
    }

    /// Returns a copy of the `Rect2i` grown by the specified `amount` on all sides.
    ///
    /// `amount` may be negative, but care must be taken: If the resulting `size` has
    /// negative components the computation may be incorrect.
    #[inline]
    pub fn grow(self, amount: i32) -> Self {
        let amount_2d = Vector2i::new(amount, amount);
        Self::from_corners(self.position - amount_2d, self.end() + amount_2d)
    }

    /// Returns a copy of the `Rect2i` grown by the specified amount on each side individually.
    ///
    /// The individual amounts may be negative, but care must be taken: If the resulting `size` has
    /// negative components the computation may be incorrect.
    #[inline]
    pub fn grow_individual(self, left: i32, top: i32, right: i32, bottom: i32) -> Self {
        let top_left = Vector2i::new(left, top);
        let bottom_right = Vector2i::new(right, bottom);
        Self::from_corners(self.position - top_left, self.end() + bottom_right)
    }

    /// Returns a copy of the `Rect2i` grown by the specified `amount` on the specified `RectSide`.
    ///
    /// `amount` may be negative, but care must be taken: If the resulting `size` has
    /// negative components the computation may be incorrect.
    #[inline]
    pub fn grow_side(self, side: RectSide, amount: i32) -> Self {
        match side {
            RectSide::Left => self.grow_individual(amount, 0, 0, 0),
            RectSide::Top => self.grow_individual(0, amount, 0, 0),
            RectSide::Right => self.grow_individual(0, 0, amount, 0),
            RectSide::Bottom => self.grow_individual(0, 0, 0, amount),
        }
    }

    /// Returns `true` if the `Rect2i` has area, and `false` if the `Rect2i` is linear, empty, or
    /// has a negative `size`.
    #[inline]
    pub const fn has_area(&self) -> bool {
        self.size.x > 0 && self.size.y > 0
    }

    /// Returns `true` if the `Rect2i` contains a point. By convention, the right and bottom edges
    /// of the `Rect2i` are considered exclusive, so points on these edges are not included.
    ///
    /// _Godot equivalent: `Rect2i.has_point` function_
    #[doc(alias = "has_point")]
    #[inline]
    pub const fn contains_point(&self, point: Vector2i) -> bool {
        self.assert_nonnegative();

        let end = self.end();
        point.x >= self.position.x
            && point.y >= self.position.y
            && point.x < end.x
            && point.y < end.y
    }

    /// Returns the intersection of this `Rect2i` and `b`.
    ///
    /// If the rectangles do not intersect, `None` is returned.
    ///
    /// Note that rectangles that only share a border do not intersect.
    #[inline]
    pub fn intersection(self, b: Self) -> Option<Self> {
        self.assert_nonnegative();
        b.assert_nonnegative();

        let own_end = self.end();
        let b_end = b.end();
        if self.position.x >= b_end.x
            || own_end.x <= b.position.x
            || self.position.y >= b_end.y
            || own_end.y <= b.position.y
        {
            return None;
        }

        let new_pos = b.position.coord_max(self.position);
        let new_end = b_end.coord_min(own_end);

        Some(Self::from_corners(new_pos, new_end))
    }

    /// Returns `true` if the `Rect2i` overlaps with `b` (i.e. they have at least one
    /// point in common)
    #[inline]
    pub fn intersects(&self, b: Self) -> bool {
        self.intersection(b).is_some()
    }

    /// Returns a larger `Rect2i` that contains this `Rect2i` and `b`.
    #[inline]
    pub fn merge(self, b: Self) -> Self {
        self.assert_nonnegative();
        b.assert_nonnegative();

        let new_pos = b.position.coord_min(self.position);
        let new_end = b.end().coord_max(self.end());

        Self::from_corners(new_pos, new_end)
    }

    /// Returns `true` if either of the coordinates of this `Rect2i`s `size` vector is negative.
    #[inline]
    pub const fn is_negative(&self) -> bool {
        self.size.x < 0 || self.size.y < 0
    }

    /// Assert that the size of the `Rect2i` is not negative.
    ///
    /// Certain functions will fail to give a correct result if the size is negative.
    #[inline]
    pub const fn assert_nonnegative(&self) {
        assert!(
            !self.is_negative(),
            "Rect2i size is negative" /* Uncomment once formatting in const contexts is allowed.
                                         Currently:
                                         error[E0015]: cannot call non-const formatting macro in constant functions
                                      "size {:?} is negative",
                                      self.size
                                      */
        );
    }
}

// SAFETY:
// This type is represented as `Self` in Godot, so `*mut Self` is sound.
unsafe impl GodotFfi for Rect2i {
    ffi_methods! { type sys::GDExtensionTypePtr = *mut Self; .. }
}

impl std::fmt::Display for Rect2i {
    /// Formats `Rect2i` to match Godot's string representation.
    ///
    /// Example:
    /// ```
    /// use godot::prelude::*;
    /// let rect = Rect2i::new(Vector2i::new(0, 0), Vector2i::new(1, 1));
    /// assert_eq!(format!("{}", rect), "[P: (0, 0), S: (1, 1)]");
    /// ```
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[P: {}, S: {}]", self.position, self.size)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn construction_tests() {
        let zero = Rect2i::default();
        let new = Rect2i::new(Vector2i::new(0, 100), Vector2i::new(1280, 720));
        let from_components = Rect2i::from_components(0, 100, 1280, 720);
        let from_rect2 = Rect2i::from_rect2(Rect2::from_components(0.1, 100.3, 1280.1, 720.42));
        let from_corners = Rect2i::from_corners(Vector2i::new(0, 100), Vector2i::new(1280, 820));

        assert_eq!(zero.position.x, 0);
        assert_eq!(zero.position.y, 0);
        assert_eq!(zero.size.x, 0);
        assert_eq!(zero.size.y, 0);

        assert_eq!(new, from_components);
        assert_eq!(new, from_rect2);
        assert_eq!(new, from_corners);

        assert_eq!(from_components, from_rect2);
        assert_eq!(from_components, from_corners);

        assert_eq!(from_rect2, from_corners);
    }

    #[test]
    fn end() {
        let rect = Rect2i::from_components(1, 2, 3, 4);
        assert_eq!(rect.end(), Vector2i::new(4, 6));

        let rect = Rect2i::from_components(1, 2, 0, 0);
        assert_eq!(rect.end(), rect.position);
    }

    #[test]
    fn set_end() {
        let mut old = Rect2i::from_components(1, 2, 3, 4);
        let new = Rect2i::from_components(1, 2, 4, 4);

        old.set_end(Vector2i::new(5, 6));
        assert_eq!(old, new);

        old.set_end(old.position);
        assert_eq!(old.end(), old.position);
    }

    #[test]
    fn abs() {
        let rect = Rect2i::from_components(1, 2, -3, -4);
        let abs = rect.abs();
        assert_eq!(abs.position.x, -2);
        assert_eq!(abs.position.y, -2);
        assert_eq!(abs.size.x, 3);
        assert_eq!(abs.size.y, 4);

        let new_abs = abs.abs();
        assert_eq!(abs, new_abs);
    }

    #[test]
    fn encloses() {
        let a = Rect2i::from_components(0, 0, 10, 10);
        let b = Rect2i::from_components(4, 4, 1, 1);
        let c = Rect2i::from_components(8, 8, 2, 2);
        let d = Rect2i::from_components(8, 8, 2, 3);

        assert!(a.encloses(a));
        assert!(a.encloses(b));
        assert!(a.encloses(c));
        assert!(!a.encloses(d));

        assert!(!b.encloses(a));
        assert!(b.encloses(b));
        assert!(!b.encloses(c));
        assert!(!b.encloses(d));

        assert!(!c.encloses(a));
        assert!(!c.encloses(b));
        assert!(c.encloses(c));
        assert!(!c.encloses(d));

        assert!(!d.encloses(a));
        assert!(!d.encloses(b));
        assert!(d.encloses(c));
        assert!(d.encloses(d));
    }

    #[test]
    #[should_panic]
    fn encloses_self_negative_panics() {
        let rect = Rect2i::from_components(0, 0, -5, -5);
        rect.encloses(Rect2i::default());
    }

    #[test]
    #[should_panic]
    fn encloses_other_negative_panics() {
        let rect = Rect2i::from_components(0, 0, -5, -5);
        Rect2i::default().encloses(rect);
    }

    #[test]
    fn expand_and_contains_point() {
        let rect = Rect2i::from_components(0, 0, 0, 0);
        let a = Vector2i::new(0, 0);
        let b = Vector2i::new(5, 0);
        let c = Vector2i::new(0, 5);
        let d = Vector2i::new(4, 4);

        assert!(!rect.contains_point(a));
        assert!(!rect.contains_point(b));
        assert!(!rect.contains_point(c));
        assert!(!rect.contains_point(d));

        let rect = rect.expand(a);

        // Note: expanding to a point does not necessarily include containing that point!
        assert!(!rect.contains_point(a));
        assert!(!rect.contains_point(b));
        assert!(!rect.contains_point(c));
        assert!(!rect.contains_point(d));

        let rect = rect.expand(b);
        assert!(!rect.contains_point(a));
        assert!(!rect.contains_point(b));
        assert!(!rect.contains_point(c));
        assert!(!rect.contains_point(d));

        let rect = rect.expand(c);
        assert!(rect.contains_point(a));
        assert!(!rect.contains_point(b));
        assert!(!rect.contains_point(c));
        assert!(rect.contains_point(d));

        let new_rect = rect.expand(d);
        assert_eq!(rect, new_rect);
    }

    #[test]
    #[should_panic]
    fn expand_self_negative_panics() {
        let rect = Rect2i::from_components(0, 0, -5, -5);
        rect.expand(Vector2i::ZERO);
    }

    #[test]
    #[should_panic]
    fn contains_point_self_negative_panics() {
        let rect = Rect2i::from_components(0, 0, -5, -5);
        rect.contains_point(Vector2i::ZERO);
    }

    #[test]
    fn area_and_has_area() {
        let a = Rect2i::from_components(0, 0, 10, 10);
        let b = Rect2i::from_components(4, 4, 1, 1);
        let c = Rect2i::from_components(8, 8, 2, 0);
        let d = Rect2i::from_components(8, 8, 0, 3);

        assert!(a.has_area());
        assert_eq!(a.area(), 100);
        assert!(b.has_area());
        assert_eq!(b.area(), 1);
        assert!(!c.has_area());
        assert_eq!(c.area(), 0);
        assert!(!d.has_area());
        assert_eq!(d.area(), 0);
    }

    #[test]
    fn center() {
        let a = Rect2i::from_components(0, 0, 10, 10);
        let b = Rect2i::from_components(4, 4, 1, 1);
        let c = Rect2i::from_components(8, 8, 2, 0);
        let d = Rect2i::from_components(8, 8, 0, 3);

        assert_eq!(a.center(), Vector2i::new(5, 5));
        assert_eq!(b.center(), Vector2i::new(4, 4));
        assert_eq!(c.center(), Vector2i::new(9, 8));
        assert_eq!(d.center(), Vector2i::new(8, 9));
    }

    #[test]
    fn grow() {
        let a = Rect2i::from_components(3, 3, 4, 4);
        let b = Rect2i::from_components(0, 0, 10, 10);
        let c = Rect2i::from_components(-3, -3, 16, 16);

        assert_eq!(a.grow(3), b);
        assert_eq!(b.grow(3), c);
        assert_eq!(a.grow(6), c);

        assert_eq!(a.grow(0), a);
        assert_eq!(b.grow(0), b);
        assert_eq!(c.grow(0), c);

        assert_eq!(c.grow(-3), b);
        assert_eq!(b.grow(-3), a);
        assert_eq!(c.grow(-6), a);
    }

    #[test]
    fn grow_individual_and_side() {
        let begin = Rect2i::from_components(3, 3, 4, 4);
        let end = Rect2i::from_components(0, 0, 10, 10);

        assert_ne!(begin, end);
        assert!(end.encloses(begin));

        let now = begin.grow_individual(3, 0, 0, 0);
        let now_side = begin.grow_side(RectSide::Left, 3);
        assert_ne!(now, end);
        assert_eq!(now, now_side);
        assert!(end.encloses(now));

        let now = now.grow_individual(0, 3, 0, 0);
        let now_side = now_side.grow_side(RectSide::Top, 3);
        assert_ne!(now, end);
        assert_eq!(now, now_side);
        assert!(end.encloses(now));

        let now = now.grow_individual(0, 0, 3, 0);
        let now_side = now_side.grow_side(RectSide::Right, 3);
        assert_ne!(now, end);
        assert_eq!(now, now_side);
        assert!(end.encloses(now));

        let now = now.grow_individual(0, 0, 0, 3);
        let now_side = now_side.grow_side(RectSide::Bottom, 3);
        assert_eq!(now, end);
        assert_eq!(now, now_side);
    }

    #[test]
    fn intersects_and_intersection() {
        let a = Rect2i::from_components(0, 0, 10, 10);
        let b = Rect2i::from_components(4, 4, 1, 1);
        let c = Rect2i::from_components(8, 8, 2, 2);
        let d = Rect2i::from_components(8, 8, 2, 3);

        assert!(a.intersects(b));
        assert_eq!(a.intersection(b), Some(b));
        assert!(a.intersects(c));
        assert_eq!(a.intersection(c), Some(c));
        assert!(a.intersects(d));
        assert_eq!(a.intersection(d), Some(c));

        assert!(!b.intersects(c));
        assert_eq!(b.intersection(c), None);
        assert!(!b.intersects(d));
        assert_eq!(b.intersection(d), None);

        assert!(c.intersects(d));
        assert_eq!(c.intersection(d), Some(c));
    }

    #[test]
    #[should_panic]
    fn intersects_self_negative_panics() {
        let rect = Rect2i::from_components(0, 0, -5, -5);
        rect.intersects(Rect2i::default());
    }

    #[test]
    #[should_panic]
    fn intersects_other_negative_panics() {
        let rect = Rect2i::from_components(0, 0, -5, -5);
        Rect2i::default().intersects(rect);
    }

    #[test]
    fn merge() {
        let a = Rect2i::from_components(0, 0, 10, 10);
        let b = Rect2i::from_components(4, 4, 1, 1);
        let c = Rect2i::from_components(8, 8, 2, 2);
        let d = Rect2i::from_components(8, 8, 2, 3);

        assert_eq!(a.merge(b), a);
        assert_eq!(a.merge(c), a);
        assert_eq!(a.merge(d), Rect2i::from_components(0, 0, 10, 11));

        assert_eq!(b.merge(c), Rect2i::from_components(4, 4, 6, 6));
        assert_eq!(b.merge(d), Rect2i::from_components(4, 4, 6, 7));

        assert_eq!(c.merge(d), d);
    }

    #[test]
    #[should_panic]
    fn merge_self_negative_panics() {
        let rect = Rect2i::from_components(0, 0, -5, -5);
        rect.merge(Rect2i::default());
    }

    #[test]
    #[should_panic]
    fn merge_other_negative_panics() {
        let rect = Rect2i::from_components(0, 0, -5, -5);
        Rect2i::default().merge(rect);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_roundtrip() {
        let rect = Rect2i::default();
        let expected_json = "{\"position\":{\"x\":0,\"y\":0},\"size\":{\"x\":0,\"y\":0}}";

        crate::builtin::test_utils::roundtrip(&rect, expected_json);
    }
}
