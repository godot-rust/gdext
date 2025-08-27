/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot_ffi as sys;
use sys::{ffi_methods, ExtVariantType, GodotFfi};

use crate::builtin::math::ApproxEq;
use crate::builtin::{real, Plane, Vector3, Vector3Axis};

/// Axis-aligned bounding box in 3D space.
///
/// `Aabb` consists of a position, a size, and several utility functions. It is typically used for
/// fast overlap tests.
///
/// Currently most methods are only available through [`InnerAabb`](super::inner::InnerAabb).
///
/// # All bounding-box types
///
/// | Dimension | Floating-point | Integer      |
/// |-----------|----------------|--------------|
/// | 2D        | [`Rect2`]      | [`Rect2i`]   |
/// | 3D        | **`Aabb`**       |              |
///
/// [`Rect2`]: crate::builtin::Rect2
/// [`Rect2i`]: crate::builtin::Rect2i
///
/// # Godot docs
///
/// [`AABB`](https://docs.godotengine.org/en/stable/classes/class_aabb.html)
#[derive(Default, Copy, Clone, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
        // Cannot use floating point arithmetic in const functions.
        Self::new(position, end - position)
    }

    /// Returns an AABB with the same geometry, with most-negative corner as `position` and non-negative `size`.
    #[inline]
    pub fn abs(self) -> Self {
        Aabb {
            position: self.position + self.size.coord_min(Vector3::ZERO),
            size: self.size.abs(),
        }
    }

    /// Whether `self` covers at least the entire area of `b` (and possibly more).
    #[inline]
    pub fn encloses(self, b: Aabb) -> bool {
        let end = self.end();
        let b_end = b.end();

        b.position.x >= self.position.x
            && b.position.y >= self.position.y
            && b.position.z >= self.position.z
            && b_end.x <= end.x
            && b_end.y <= end.y
            && b_end.z <= end.z
    }

    /// Returns a copy of this AABB expanded to include a given point.
    ///
    /// # Panics
    /// If `self.size` is negative.
    #[inline]
    pub fn expand(self, to: Vector3) -> Self {
        self.merge(Aabb::new(to, Vector3::ZERO))
    }

    /// Returns a larger AABB that contains this AABB and `b`.
    ///
    /// # Panics
    /// If either `self.size` or `b.size` is negative.
    #[inline]
    pub fn merge(self, b: Aabb) -> Self {
        self.assert_nonnegative();
        b.assert_nonnegative();

        let position = self.position.coord_min(b.position);
        let end = self.end().coord_max(b.end());

        Self::from_corners(position, end)
    }

    /// Returns the volume of the AABB.
    ///
    /// # Panics
    /// If `self.size` is negative.
    #[inline]
    pub fn volume(self) -> real {
        self.assert_nonnegative();
        self.size.x * self.size.y * self.size.z
    }

    /// Returns the center of the AABB, which is equal to `position + (size / 2)`.
    #[inline]
    pub fn center(self) -> Vector3 {
        self.position + (self.size / 2.0)
    }

    /// Returns a copy of the AABB grown by the specified `amount` on all sides.
    #[inline]
    #[must_use]
    pub fn grow(self, amount: real) -> Self {
        let position = self.position - Vector3::new(amount, amount, amount);
        let size = self.size + Vector3::new(amount, amount, amount) * 2.0;

        Self { position, size }
    }

    /// Returns `true` if the AABB contains a point (excluding right/bottom edge).
    ///
    /// By convention, the right and bottom edges of the AABB are considered exclusive, so points on these edges are not included.
    ///
    /// # Panics
    /// If `self.size` is negative.
    #[inline]
    #[doc(alias = "has_point")]
    pub fn contains_point(self, point: Vector3) -> bool {
        self.assert_nonnegative();

        let point = point - self.position;

        point.abs() == point
            && point.x < self.size.x
            && point.y < self.size.y
            && point.z < self.size.z
    }

    /// Returns if this bounding box has a surface or a length, i.e. at least one component of [`Self::size`] is greater than 0.
    #[inline]
    pub fn has_surface(self) -> bool {
        (self.size.x > 0.0) || (self.size.y > 0.0) || (self.size.z > 0.0)
    }

    /// Returns true if the AABB has a volume, and false if the AABB is flat, linear, empty, or has a negative size.
    #[inline]
    pub fn has_volume(self) -> bool {
        self.size.x > 0.0 && self.size.y > 0.0 && self.size.z > 0.0
    }

    /// Returns the intersection between two AABBs.
    ///
    /// # Panics (Debug)
    /// If `self.size` is negative.
    #[inline]
    pub fn intersect(self, b: Aabb) -> Option<Self> {
        self.assert_nonnegative();

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

    /// Returns `true` if this AABB is finite, by calling `@GlobalScope.is_finite` on each component.
    #[inline]
    pub fn is_finite(self) -> bool {
        self.position.is_finite() && self.size.is_finite()
    }

    /// The end of the `Aabb` calculated as `position + size`.
    #[inline]
    pub fn end(self) -> Vector3 {
        self.position + self.size
    }

    /// Set size based on desired end-point.
    ///
    /// NOTE: This does not make the AABB absolute, and `Aabb.abs()` should be called if the size becomes negative.
    #[inline]
    pub fn set_end(&mut self, end: Vector3) {
        self.size = end - self.position
    }

    /// Returns the normalized longest axis of the AABB.
    #[inline]
    pub fn longest_axis(self) -> Option<Vector3> {
        self.longest_axis_index().map(|axis| match axis {
            Vector3Axis::X => Vector3::RIGHT,
            Vector3Axis::Y => Vector3::UP,
            Vector3Axis::Z => Vector3::BACK,
        })
    }

    /// Returns the index of the longest axis of the AABB (according to Vector3's AXIS_* constants).
    #[inline]
    pub fn longest_axis_index(self) -> Option<Vector3Axis> {
        self.size.max_axis()
    }

    /// Returns the scalar length of the longest axis of the AABB.
    #[inline]
    pub fn longest_axis_size(self) -> real {
        let size = self.size;
        size.x.max(size.y).max(size.z)
    }

    /// Returns the normalized shortest axis of the AABB.
    #[inline]
    pub fn shortest_axis(self) -> Option<Vector3> {
        self.shortest_axis_index().map(|axis| match axis {
            Vector3Axis::X => Vector3::RIGHT,
            Vector3Axis::Y => Vector3::UP,
            Vector3Axis::Z => Vector3::BACK,
        })
    }

    /// Returns the index of the shortest axis of the AABB (according to Vector3::AXIS* enum).
    #[inline]
    pub fn shortest_axis_index(self) -> Option<Vector3Axis> {
        self.size.min_axis()
    }

    /// Returns the scalar length of the shortest axis of the AABB.
    #[inline]
    pub fn shortest_axis_size(self) -> real {
        self.size.x.min(self.size.y.min(self.size.z))
    }

    /// Returns the support point in a given direction. This is useful for collision detection algorithms.
    #[inline]
    #[doc(alias = "get_support")]
    pub fn support(self, dir: Vector3) -> Vector3 {
        let half_extents = self.size * 0.5;
        let relative_center_point = self.position + half_extents;

        let signs = Vector3 {
            x: dir.x.signum(),
            y: dir.y.signum(),
            z: dir.z.signum(),
        };

        half_extents * signs + relative_center_point
    }

    /// Checks whether two AABBs have at least one point in common.
    ///
    /// Also returns `true` if the AABBs only touch each other (share a point/edge/face).
    /// See [`intersects_exclude_borders`][Self::intersects_exclude_borders] if you want to return `false` in that case.
    ///
    /// _Godot equivalent: `AABB.intersects(AABB b, bool include_borders = true)`_
    #[inline]
    pub fn intersects(self, b: Aabb) -> bool {
        let end = self.end();
        let end_b = b.end();

        self.position.x <= end_b.x
            && end.x >= b.position.x
            && self.position.y <= end_b.y
            && end.y >= b.position.y
            && self.position.z <= end_b.z
    }

    /// Checks whether two AABBs have at least one _inner_ point in common (not on the borders).
    ///
    /// Returns `false` if the AABBs only touch each other (share a point/edge/face).
    /// See [`intersects`][Self::intersects] if you want to return `true` in that case.
    ///
    /// _Godot equivalent: `AABB.intersects(AABB b, bool include_borders = false)`_
    #[inline]
    pub fn intersects_exclude_borders(self, b: Aabb) -> bool {
        let end = self.end();
        let end_b = b.end();

        self.position.x < end_b.x
            && end.x > b.position.x
            && self.position.y < end_b.y
            && end.y > b.position.y
            && self.position.z < end_b.z
            && end.z > b.position.z
    }

    /// Returns `true` if the AABB is on both sides of a plane.
    #[inline]
    pub fn intersects_plane(self, plane: Plane) -> bool {
        // The set of the edges of the AABB.
        let points = [
            self.position,
            self.position + Vector3::new(0.0, 0.0, self.size.z),
            self.position + Vector3::new(0.0, self.size.y, 0.0),
            self.position + Vector3::new(self.size.x, 0.0, 0.0),
            self.position + Vector3::new(self.size.x, self.size.y, 0.0),
            self.position + Vector3::new(self.size.x, 0.0, self.size.z),
            self.position + Vector3::new(0.0, self.size.y, self.size.z),
            self.position + self.size,
        ];

        let mut over = false;
        let mut under = false;

        for point in points {
            let dist_to = plane.distance_to(point);
            if dist_to > 0.0 {
                over = true
            } else {
                under = true
            }
        }

        over && under
    }

    /// Returns `true` if the given ray intersects with this AABB. Ray length is infinite.
    ///
    /// Semantically equivalent to `self.intersects_ray(ray_from, ray_dir).is_some()`; might be microscopically faster.
    ///
    /// # Panics (Debug)
    /// If `self.size` is negative.
    #[inline]
    pub fn intersects_ray(self, ray_from: Vector3, ray_dir: Vector3) -> bool {
        let (tnear, tfar) = self.compute_ray_tnear_tfar(ray_from, ray_dir);

        tnear <= tfar
    }

    /// Returns the point where the given (infinite) ray intersects with this AABB, or `None` if there is no intersection.
    ///
    /// # Panics (Debug)
    /// If `self.size` is negative, or if `ray_dir` is zero. Note that this differs from Godot, which treats rays that degenerate to points as
    /// intersecting if inside, and not if outside the AABB.
    #[inline]
    pub fn intersect_ray(self, ray_from: Vector3, ray_dir: Vector3) -> Option<Vector3> {
        let (tnear, tfar) = self.compute_ray_tnear_tfar(ray_from, ray_dir);

        if tnear <= tfar {
            // if tnear < 0: the ray starts inside the box -> take other intersection point.
            let t = if tnear < 0.0 { tfar } else { tnear };
            Some(ray_from + ray_dir * t)
        } else {
            None
        }
    }

    // Credits: https://tavianator.com/2011/ray_box.html
    fn compute_ray_tnear_tfar(self, ray_from: Vector3, ray_dir: Vector3) -> (real, real) {
        self.assert_nonnegative();
        debug_assert!(
            ray_dir != Vector3::ZERO,
            "ray direction must not be zero; use contains_point() for point checks"
        );

        // Note: leads to -inf/inf for each component that is 0. This should generally balance out, unless all are zero.
        let recip_dir = ray_dir.recip();

        let tmin = (self.position - ray_from) * recip_dir;
        let tmax = (self.end() - ray_from) * recip_dir;

        let t1 = tmin.coord_min(tmax);
        let t2 = tmin.coord_max(tmax);

        let tnear = t1.x.max(t1.y).max(t1.z);
        let tfar = t2.x.min(t2.y).min(t2.z);

        (tnear, tfar)
    }

    /// Returns `true` if the given ray intersects with this AABB. Segment length is finite.
    ///
    /// # Panics
    /// If `self.size` is negative.
    #[inline]
    pub fn intersects_segment(self, from: Vector3, to: Vector3) -> bool {
        self.assert_nonnegative();

        let segment_dir = to - from;

        let mut t_min: real = 0.0;
        let mut t_max: real = 1.0;

        for axis in [Vector3Axis::X, Vector3Axis::Y, Vector3Axis::Z] {
            let inv_dir = 1.0 / segment_dir[axis];

            let t1 = (self.position[axis] - from[axis]) * inv_dir;
            let t2 = (self.end()[axis] - from[axis]) * inv_dir;

            let (t_near, t_far) = if t1 < t2 { (t1, t2) } else { (t2, t1) };

            // Update t_min and t_max
            t_min = t_min.max(t_near);
            t_max = t_max.min(t_far);

            if t_min > t_max {
                // No intersection or segment completely outside the AABB
                return false;
            }
        }

        true
    }

    /// Assert that the size of the `Aabb` is not negative.
    ///
    /// Most functions will fail to give a correct result if the size is negative.
    #[inline]
    /// TODO(v0.3): make private, change to debug_assert().
    pub fn assert_nonnegative(self) {
        assert!(
            self.size.x >= 0.0 && self.size.y >= 0.0 && self.size.z >= 0.0,
            "size {:?} is negative",
            self.size
        );
    }
}

impl std::fmt::Display for Aabb {
    /// Formats `Aabb` to match godot's display style.
    ///
    /// # Example
    /// ```
    /// use godot::prelude::*;
    /// let aabb = Aabb::new(Vector3::new(0.0, 0.0, 0.0), Vector3::new(1.0, 1.0, 1.0));
    /// assert_eq!(format!("{}", aabb), "[P: (0, 0, 0), S: (1, 1, 1)]");
    /// ```
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[P: {}, S: {}]", self.position, self.size)
    }
}

// SAFETY:
// This type is represented as `Self` in Godot, so `*mut Self` is sound.
unsafe impl GodotFfi for Aabb {
    const VARIANT_TYPE: ExtVariantType = ExtVariantType::Concrete(sys::VariantType::AABB);

    ffi_methods! { type sys::GDExtensionTypePtr = *mut Self; .. }
}

crate::meta::impl_godot_as_self!(Aabb: ByValue);

impl ApproxEq for Aabb {
    /// Returns `true` if the two `Aabb`s are approximately equal, by calling `is_equal_approx` on
    /// `position` and `size`.
    #[inline]
    fn approx_eq(&self, other: &Self) -> bool {
        Vector3::approx_eq(&self.position, &other.position)
            && Vector3::approx_eq(&self.size, &other.size)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[cfg(feature = "serde")]
    #[test]
    fn serde_roundtrip() {
        let aabb = super::Aabb::default();
        let expected_json = "{\"position\":{\"x\":0.0,\"y\":0.0,\"z\":0.0},\"size\":{\"x\":0.0,\"y\":0.0,\"z\":0.0}}";

        crate::builtin::test_utils::roundtrip(&aabb, expected_json);
    }

    #[test]
    fn test_axes_functions() {
        let aabb = Aabb {
            position: Vector3::new(0.0, 0.0, 0.0),
            size: Vector3::new(4.0, 6.0, 8.0),
        };

        assert_eq!(aabb.shortest_axis(), Some(Vector3::RIGHT));
        assert_eq!(aabb.longest_axis(), Some(Vector3::BACK));
        assert_eq!(aabb.shortest_axis_size(), 4.0);
        assert_eq!(aabb.longest_axis_size(), 8.0);
        assert_eq!(aabb.shortest_axis_index(), Some(Vector3Axis::X));
        assert_eq!(aabb.longest_axis_index(), Some(Vector3Axis::Z));
    }

    #[test]
    fn test_intersects() {
        let aabb1 = Aabb {
            position: Vector3::new(0.0, 0.0, 0.0),
            size: Vector3::new(4.0, 4.0, 4.0),
        };

        let aabb2 = Aabb {
            position: Vector3::new(3.0, 3.0, 3.0),
            size: Vector3::new(3.0, 3.0, 3.0),
        };

        let aabb3 = Aabb {
            position: Vector3::new(5.0, 5.0, 5.0),
            size: Vector3::new(2.0, 2.0, 2.0),
        };

        let aabb4 = Aabb {
            position: Vector3::new(6.0, 6.0, 6.0),
            size: Vector3::new(1.0, 1.0, 1.0),
        };

        // Check for intersection including border.
        assert!(aabb1.intersects(aabb2));
        assert!(aabb2.intersects(aabb1));

        // Check for non-intersection including border.
        assert!(!aabb1.intersects(aabb3));
        assert!(!aabb3.intersects(aabb1));

        // Check for intersection excluding border.
        assert!(aabb1.intersects_exclude_borders(aabb2));
        assert!(aabb2.intersects_exclude_borders(aabb1));

        // Check for non-intersection excluding border.
        assert!(!aabb1.intersects_exclude_borders(aabb3));
        assert!(!aabb3.intersects_exclude_borders(aabb1));

        // Check for non-intersection excluding border.
        assert!(!aabb1.intersects_exclude_borders(aabb4));
        assert!(!aabb4.intersects_exclude_borders(aabb1));

        // Check for intersection with same AABB including border.
        assert!(aabb1.intersects(aabb1));
    }

    #[test]
    fn test_intersection() {
        // Create AABBs for testing
        let aabb1 = Aabb {
            position: Vector3::new(0.0, 0.0, 0.0),
            size: Vector3::new(2.0, 2.0, 2.0),
        };

        let aabb2 = Aabb {
            position: Vector3::new(1.0, 1.0, 1.0),
            size: Vector3::new(2.0, 2.0, 2.0),
        };

        let aabb3 = Aabb {
            position: Vector3::new(3.0, 3.0, 3.0),
            size: Vector3::new(2.0, 2.0, 2.0),
        };

        let aabb4 = Aabb {
            position: Vector3::new(-1.0, -1.0, -1.0),
            size: Vector3::new(1.0, 1.0, 1.0),
        };

        assert_eq!(
            aabb1.intersect(aabb2),
            Some(Aabb {
                position: Vector3::new(1.0, 1.0, 1.0),
                size: Vector3::new(1.0, 1.0, 1.0),
            })
        );

        assert_eq!(aabb1.intersect(aabb3), None);

        assert_eq!(
            aabb1.intersect(aabb4),
            Some(Aabb {
                position: Vector3::new(0.0, 0.0, 0.0),
                size: Vector3::new(0.0, 0.0, 0.0),
            })
        );
    }

    #[test]
    fn test_intersects_ray() {
        // Test case 1: Ray intersects the AABB.
        let aabb1 = Aabb {
            position: Vector3::new(0.0, 0.0, 0.0),
            size: Vector3::new(2.0, 2.0, 2.0),
        };
        let from1 = Vector3::new(1.0, 1.0, -1.0);
        let dir1 = Vector3::new(0.0, 0.0, 1.0);

        assert!(aabb1.intersects_ray(from1, dir1));

        // Test case 2: Ray misses the AABB.
        let aabb2 = Aabb {
            position: Vector3::new(0.0, 0.0, 0.0),
            size: Vector3::new(2.0, 2.0, 2.0),
        };
        let from2 = Vector3::new(4.0, 4.0, 4.0);
        let dir2 = Vector3::new(0.0, 0.0, 1.0);
        assert!(!aabb2.intersects_ray(from2, dir2));

        // Test case 3: Ray starts inside the AABB.
        let aabb3 = Aabb {
            position: Vector3::new(0.0, 0.0, 0.0),
            size: Vector3::new(2.0, 2.0, 2.0),
        };
        let from3 = Vector3::new(1.0, 1.0, 1.0);
        let dir3 = Vector3::new(0.0, 0.0, 1.0);
        assert!(aabb3.intersects_ray(from3, dir3));

        // Test case 4: Ray direction parallel to AABB.
        let aabb4 = Aabb {
            position: Vector3::new(0.0, 0.0, 0.0),
            size: Vector3::new(2.0, 2.0, 2.0),
        };
        let from4 = Vector3::new(1.0, 1.0, 1.0);
        let dir4 = Vector3::new(1.0, 0.0, 0.0);
        assert!(aabb4.intersects_ray(from4, dir4));

        // Test case 5: Ray direction diagonal through the AABB.
        let aabb5 = Aabb {
            position: Vector3::new(0.0, 0.0, 0.0),
            size: Vector3::new(2.0, 2.0, 2.0),
        };
        let from5 = Vector3::new(0.5, 0.5, 0.5);
        let dir5 = Vector3::new(1.0, 1.0, 1.0);
        assert!(aabb5.intersects_ray(from5, dir5));

        // Test case 6: Ray origin on an AABB face.
        let aabb6 = Aabb {
            position: Vector3::new(0.0, 0.0, 0.0),
            size: Vector3::new(2.0, 2.0, 2.0),
        };
        let from6 = Vector3::new(1.0, 2.0, 1.0);
        let dir6 = Vector3::new(0.0, -1.0, 0.0);
        assert!(aabb6.intersects_ray(from6, dir6));
    }

    #[test] // Ported from Godot tests.
    fn test_intersect_ray_2() {
        let aabb = Aabb {
            position: Vector3::new(-1.5, 2.0, -2.5),
            size: Vector3::new(4.0, 5.0, 6.0),
        };

        assert_eq!(
            aabb.intersect_ray(Vector3::new(-100.0, 3.0, 0.0), Vector3::new(1.0, 0.0, 0.0)),
            Some(Vector3::new(-1.5, 3.0, 0.0)),
            "intersect_ray(), ray points directly at AABB -> Some"
        );

        assert_eq!(
            aabb.intersect_ray(Vector3::new(10.0, 10.0, 0.0), Vector3::new(0.0, 1.0, 0.0)),
            None,
            "intersect_ray(), ray parallel and outside the AABB -> None"
        );

        assert_eq!(
            aabb.intersect_ray(Vector3::ONE, Vector3::new(0.0, 1.0, 0.0)),
            Some(Vector3::new(1.0, 2.0, 1.0)),
            "intersect_ray(), ray originating inside the AABB -> Some"
        );

        assert_eq!(
            aabb.intersect_ray(Vector3::new(-10.0, 0.0, 0.0), Vector3::new(-1.0, 0.0, 0.0)),
            None,
            "intersect_ray(), ray points away from AABB -> None"
        );

        assert_eq!(
            aabb.intersect_ray(Vector3::new(0.0, 0.0, 0.0), Vector3::ONE),
            Some(Vector3::new(2.0, 2.0, 2.0)),
            "intersect_ray(), ray along the AABB diagonal -> Some"
        );

        assert_eq!(
            aabb.intersect_ray(
                aabb.position + Vector3::splat(0.0001),
                Vector3::new(-1.0, 0.0, 0.0)
            ),
            Some(Vector3::new(-1.5, 2.0001, -2.4999)),
            "intersect_ray(), ray starting on the AABB's edge -> Some"
        );

        assert_eq!(
            aabb.intersect_ray(Vector3::new(0.0, 0.0, 0.0), Vector3::new(0.0, 1.0, 0.0)),
            Some(Vector3::new(0.0, 2.0, 0.0)),
            "intersect_ray(): ray has 2 axes parallel to AABB -> Some"
        );
    }

    #[test] // Ported from Godot tests.
    fn test_intersect_aabb() {
        let aabb_big = Aabb {
            position: Vector3::new(-1.5, 2.0, -2.5),
            size: Vector3::new(4.0, 5.0, 6.0),
        };

        let aabb_small = Aabb {
            position: Vector3::new(-1.5, 2.0, -2.5),
            size: Vector3::ONE,
        };
        assert!(
            aabb_big.intersects(aabb_small),
            "intersects() with fully contained AABB (touching the edge) should return true."
        );

        let aabb_small = Aabb {
            position: Vector3::new(0.5, 1.5, -2.0),
            size: Vector3::ONE,
        };
        assert!(
            aabb_big.intersects(aabb_small),
            "intersects() with partially contained AABB (overflowing on Y axis) should return true."
        );

        let aabb_small = Aabb {
            position: Vector3::new(10.0, -10.0, -10.0),
            size: Vector3::ONE,
        };
        assert!(
            !aabb_big.intersects(aabb_small),
            "intersects() with non-contained AABB should return false."
        );

        let aabb_small = Aabb {
            position: Vector3::new(-1.5, 2.0, -2.5),
            size: Vector3::ONE,
        };
        let inter = aabb_big.intersect(aabb_small);
        assert!(
            inter.unwrap().approx_eq(&aabb_small),
            "intersect() with fully contained AABB should return the smaller AABB."
        );

        let aabb_small = Aabb {
            position: Vector3::new(0.5, 1.5, -2.0),
            size: Vector3::ONE,
        };
        let expected = Aabb {
            position: Vector3::new(0.5, 2.0, -2.0),
            size: Vector3::new(1.0, 0.5, 1.0),
        };
        let inter = aabb_big.intersect(aabb_small);
        assert!(
            inter.unwrap().approx_eq(&expected),
            "intersect() with partially contained AABB (overflowing on Y axis) should match expected."
        );

        let aabb_small = Aabb {
            position: Vector3::new(10.0, -10.0, -10.0),
            size: Vector3::ONE,
        };
        let inter = aabb_big.intersect(aabb_small);
        assert!(
            inter.is_none(),
            "intersect() with non-contained AABB should return None."
        );
    }

    #[test]
    #[should_panic]
    #[cfg(debug_assertions)]
    fn test_intersect_ray_zero_dir_inside() {
        let aabb = Aabb {
            position: Vector3::new(-1.5, 2.0, -2.5),
            size: Vector3::new(4.0, 5.0, 6.0),
        };

        aabb.intersect_ray(Vector3::new(-1.0, 3.0, -2.0), Vector3::ZERO);
    }

    #[test]
    #[should_panic]
    #[cfg(debug_assertions)]
    fn test_intersect_ray_zero_dir_outside() {
        let aabb = Aabb {
            position: Vector3::new(-1.5, 2.0, -2.5),
            size: Vector3::new(4.0, 5.0, 6.0),
        };

        aabb.intersect_ray(Vector3::new(-1000.0, 3.0, -2.0), Vector3::ZERO);
    }

    #[test]
    fn test_intersects_plane() {
        let aabb = Aabb {
            position: Vector3::new(-1.0, -1.0, -1.0),
            size: Vector3::new(2.0, 2.0, 2.0),
        };

        let plane_inside = Plane {
            normal: Vector3::new(1.0, 0.0, 0.0),
            d: 0.0,
        };

        let plane_outside = Plane {
            normal: Vector3::new(1.0, 0.0, 0.0),
            d: 2.0,
        };

        let plane_intersect = Plane {
            normal: Vector3::new(0.0, 1.0, 0.0),
            d: 0.5,
        };

        let plane_parallel = Plane {
            normal: Vector3::new(0.0, 1.0, 0.0),
            d: 2.0,
        };

        // Test cases
        assert!(aabb.intersects_plane(plane_inside));
        assert!(!aabb.intersects_plane(plane_outside));
        assert!(aabb.intersects_plane(plane_intersect));
        assert!(!aabb.intersects_plane(plane_parallel));
    }

    #[test] // Ported from Godot tests.
    fn test_intersects_plane_2() {
        let aabb_big = Aabb {
            position: Vector3::new(-1.5, 2.0, -2.5),
            size: Vector3::new(4.0, 5.0, 6.0),
        };

        let plane1 = Plane::new(Vector3::new(0.0, 1.0, 0.0), 4.0);
        assert!(
            aabb_big.intersects_plane(plane1),
            "intersects_plane() should return true (plane near top)."
        );

        let plane2 = Plane::new(Vector3::new(0.0, -1.0, 0.0), -4.0);
        assert!(
            aabb_big.intersects_plane(plane2),
            "intersects_plane() should return true (plane near bottom)."
        );

        let plane3 = Plane::new(Vector3::new(0.0, 1.0, 0.0), 200.0);
        assert!(
            !aabb_big.intersects_plane(plane3),
            "intersects_plane() should return false (plane far away)."
        );
    }

    #[test]
    fn test_aabb_intersects_segment() {
        let aabb = Aabb {
            position: Vector3::new(0.0, 0.0, 0.0),
            size: Vector3::new(4.0, 4.0, 4.0),
        };

        // Test case: Segment intersects AABB
        let from = Vector3::new(1.0, 1.0, 1.0);
        let to = Vector3::new(3.0, 3.0, 3.0);
        assert!(aabb.intersects_segment(from, to));

        // Test case: Segment does not intersect AABB
        let from = Vector3::new(-2.0, 2.0, 2.0);
        let to = Vector3::new(-1.0, 1.0, 1.0);
        assert!(!aabb.intersects_segment(from, to));
    }

    #[test] // Ported from Godot tests.
    fn test_intersects_segment_2() {
        let aabb = Aabb {
            position: Vector3::new(-1.5, 2.0, -2.5),
            size: Vector3::new(4.0, 5.0, 6.0),
        };

        // True cases.
        assert!(
            aabb.intersects_segment(Vector3::new(1.0, 3.0, 0.0), Vector3::new(0.0, 3.0, 0.0)),
            "intersects_segment(), segment fully inside -> true"
        );
        assert!(
            aabb.intersects_segment(Vector3::new(0.0, 3.0, 0.0), Vector3::new(0.0, -300.0, 0.0)),
            "intersects_segment(), segment crossing the box -> true"
        );
        assert!(
            aabb.intersects_segment(
                Vector3::new(-50.0, 3.0, -50.0),
                Vector3::new(50.0, 3.0, 50.0)
            ),
            "intersects_segment(), diagonal crossing the box -> true"
        );

        // False case.
        assert!(
            !aabb.intersects_segment(
                Vector3::new(-50.0, 25.0, -50.0),
                Vector3::new(50.0, 25.0, 50.0)
            ),
            "intersects_segment(), segment above the box -> false"
        );

        // Degenerate segments (points).
        assert!(
            aabb.intersects_segment(Vector3::new(0.0, 3.0, 0.0), Vector3::new(0.0, 3.0, 0.0)),
            "intersects_segment(), segment of length 0 *inside* the box -> true"
        );
        assert!(
            !aabb.intersects_segment(Vector3::new(0.0, 300.0, 0.0), Vector3::new(0.0, 300.0, 0.0)),
            "intersects_segment(), segment of length 0 *outside* the box -> false"
        );
    }
}
