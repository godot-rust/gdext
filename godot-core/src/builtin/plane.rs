/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot_ffi as sys;
use sys::{ffi_methods, GodotFfi};

use crate::builtin::math::{ApproxEq, FloatExt};
use crate::builtin::{real, Vector3};

use std::ops::Neg;

/// 3D plane in [Hessian normal form](https://mathworld.wolfram.com/HessianNormalForm.html).
///
/// The Hessian form defines all points `point` which satisfy the equation
/// `dot(normal, point) + d == 0`, where `normal` is the normal vector and `d`
/// the distance from the origin.
///
/// Note: almost all methods on `Plane` require that the `normal` vector have
/// unit length and will panic if this invariant is violated. This is not separately
/// annotated for each method.
#[derive(Copy, Clone, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(C)]
pub struct Plane {
    /// Normal vector pointing away from the plane.
    pub normal: Vector3,

    /// Distance between the plane and the origin point.
    pub d: real,
}

impl Plane {
    /// Creates a new `Plane` from the `normal` and the distance from the origin `d`.
    ///
    /// # Panics
    /// In contrast to construction via `Plane { normal, d }`, this verifies that `normal` has unit length, and will
    /// panic if this is not the case.
    ///
    /// _Godot equivalent: `Plane(Vector3 normal, float d)`_
    #[inline]
    pub fn new(unit_normal: Vector3, d: real) -> Self {
        let plane = Self {
            normal: unit_normal,
            d,
        };
        plane.assert_normalized();
        plane
    }

    /// Create a new `Plane` through the origin from a normal.
    ///
    /// # Panics
    /// See [`Self::new()`].
    ///
    /// _Godot equivalent: `Plane(Vector3 normal)`_
    #[inline]
    pub fn from_normal_at_origin(normal: Vector3) -> Self {
        Self::new(normal, 0.0)
    }

    /// Create a new `Plane` from a normal and a point in the plane.
    ///
    /// # Panics
    /// See [`Self::new()`].
    ///
    /// _Godot equivalent: `Plane(Vector3 normal, Vector3 point)`_
    #[inline]
    pub fn from_point_normal(point: Vector3, normal: Vector3) -> Self {
        Self::new(normal, normal.dot(point))
    }

    /// Creates a new `Plane` from normal and origin distance.
    ///
    /// `nx`, `ny`, `nz` are used for the `normal` vector.
    /// `d` is the distance from the origin.
    ///
    /// # Panics
    /// See [`Self::new()`].
    ///
    /// _Godot equivalent: `Plane(float a, float b, float c, float d)`_
    #[inline]
    pub fn from_components(nx: real, ny: real, nz: real, d: real) -> Self {
        Self::new(Vector3::new(nx, ny, nz), d)
    }

    /// Creates a new `Plane` from three points, given in clockwise order.
    ///
    /// # Panics
    /// Will panic if all three points are colinear.
    ///
    /// _Godot equivalent: `Plane(Vector3 point1, Vector3 point2, Vector3 point3)`_
    #[inline]
    pub fn from_points(a: Vector3, b: Vector3, c: Vector3) -> Self {
        let normal = (a - c).cross(a - b);
        assert_ne!(
            normal,
            Vector3::ZERO,
            "points {a}, {b}, {c} are all colinear"
        );
        let normal = normal.normalized();
        Self {
            normal,
            d: normal.dot(a),
        }
    }

    /// Finds the shortest distance between the plane and a point.
    ///
    /// The distance will be positive if `point` is above the plane, and will be negative if
    /// `point` is below the plane.
    #[inline]
    pub fn distance_to(&self, point: Vector3) -> real {
        self.normal.dot(point) - self.d
    }

    /// Finds the center point of the plane.
    ///
    /// _Godot equivalent: `Plane.get_center()`_
    #[inline]
    pub fn center(&self) -> Vector3 {
        self.normal * self.d
    }

    /// Finds whether a point is inside the plane or not.
    ///
    /// A point is considered part of the plane if its distance to it is less or equal than
    /// [`CMP_EPSILON`][crate::builtin::CMP_EPSILON].
    ///
    /// _Godot equivalent: `Plane.has_point(Vector3 point, float tolerance=1e-05)`_
    #[inline]
    #[doc(alias = "has_point")]
    pub fn contains_point(&self, point: Vector3, tolerance: Option<real>) -> bool {
        let dist: real = (self.normal.dot(point) - self.d).abs();
        dist <= tolerance.unwrap_or(real::CMP_EPSILON)
    }

    /// Finds the intersection point of three planes.
    ///
    /// If no intersection point is found, `None` will be returned.
    #[inline]
    pub fn intersect_3(&self, b: &Self, c: &Self) -> Option<Vector3> {
        let normal0 = self.normal;
        let normal1 = b.normal;
        let normal2 = c.normal;
        let denom: real = normal0.cross(normal1).dot(normal2);
        if denom.is_zero_approx() {
            return None;
        }
        let result = normal1.cross(normal2) * self.d
            + normal2.cross(normal0) * b.d
            + normal0.cross(normal1) * c.d;
        Some(result / denom)
    }

    /// Finds the intersection point of the plane with a ray.
    ///
    /// The ray starts at position `from` and has direction vector `dir`, i.e. it is unbounded in one direction.
    ///
    /// If no intersection is found (the ray is parallel to the plane or points away from it), `None` will be returned.
    #[inline]
    pub fn intersect_ray(&self, from: Vector3, dir: Vector3) -> Option<Vector3> {
        let denom: real = self.normal.dot(dir);
        if denom.is_zero_approx() {
            return None;
        }
        let dist: real = (self.normal.dot(from) - self.d) / denom;
        if dist > real::CMP_EPSILON {
            return None;
        }
        Some(from - dir * dist)
    }

    /// Finds the intersection point of the plane with a line segment.
    ///
    /// The segment starts at position 'from' and ends at position 'to', i.e. it is bounded at two directions.
    ///
    /// If no intersection is found (the segment is parallel to the plane or does not intersect it), `None` will be returned.
    #[inline]
    pub fn intersect_segment(&self, from: Vector3, to: Vector3) -> Option<Vector3> {
        let segment = from - to;
        let denom: real = self.normal.dot(segment);
        if denom.is_zero_approx() {
            return None;
        }
        let dist: real = (self.normal.dot(from) - self.d) / denom;
        if !(-real::CMP_EPSILON..=(1.0 + real::CMP_EPSILON)).contains(&dist) {
            return None;
        }
        Some(from - segment * dist)
    }

    /// Returns `true` if the plane is finite by calling `is_finite` on `normal` and `d`.
    #[inline]
    pub fn is_finite(&self) -> bool {
        self.normal.is_finite() && self.d.is_finite()
    }

    /// Returns `true` if `point` is located above the plane.
    #[inline]
    pub fn is_point_over(&self, point: Vector3) -> bool {
        self.normal.dot(point) > self.d
    }

    /// Returns a copy of the plane with its `normal` and `d` scaled to the unit length.
    ///
    /// A `normal` length of `0.0` would return a plane with its `normal` and `d` being `Vector3::ZERO`
    /// and `0.0` respectively.
    #[inline]
    pub fn normalized(self) -> Self {
        let length: real = self.normal.length();
        if length == 0.0 {
            return Plane {
                normal: Vector3::ZERO,
                d: 0.0,
            };
        }
        Plane::new(self.normal.normalized(), self.d / length)
    }

    /// Returns the orthogonal projection of `point` to the plane.
    #[inline]
    pub fn project(&self, point: Vector3) -> Vector3 {
        point - self.normal * self.distance_to(point)
    }

    #[inline]
    fn assert_normalized(self) {
        assert!(
            self.normal.is_normalized(),
            "normal {:?} is not normalized",
            self.normal
        );
    }
}

impl Neg for Plane {
    type Output = Plane;

    /// Returns the negative value of the plane by flipping both the normal and the distance value. Meaning
    /// it creates a plane that is in the same place, but facing the opposite direction.
    fn neg(self) -> Self::Output {
        Self::new(-self.normal, -self.d)
    }
}

// SAFETY:
// This type is represented as `Self` in Godot, so `*mut Self` is sound.
unsafe impl GodotFfi for Plane {
    ffi_methods! { type sys::GDExtensionTypePtr = *mut Self; .. }
}

impl ApproxEq for Plane {
    /// Finds whether the two planes are approximately equal.
    ///
    /// Returns if the two `Plane`s are approximately equal, by comparing `normal` and `d` separately.
    /// If one plane is a negation of the other (both `normal` and `d` have opposite signs), they are considered approximately equal.
    #[inline]
    fn approx_eq(&self, other: &Self) -> bool {
        (Vector3::approx_eq(&self.normal, &other.normal) //.
            && self.d.approx_eq(&other.d))
            || (Vector3::approx_eq(&self.normal, &(-other.normal)) && self.d.approx_eq(&-other.d))
    }
}

impl std::fmt::Display for Plane {
    /// Formats `Plane` to match Godot's string representation.
    ///
    /// Example:
    /// ```
    /// use godot::prelude::*;
    /// let plane = Plane::new(Vector3::new(1.0, 0.0, 0.0), 1.0);
    /// assert_eq!(format!("{}", plane), "[N: (1, 0, 0), D: 1]");
    /// ```
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[N: {}, D: {}]", self.normal, self.d)
    }
}

#[cfg(test)]
mod test {
    use crate::assert_eq_approx;
    use crate::assert_ne_approx;

    use super::*;

    /// Tests that none of the constructors panic for some simple planes.
    #[test]
    fn construction_succeeds() {
        let vec = Vector3::new(1.0, 2.0, 3.0).normalized();
        let Vector3 { x, y, z } = vec;
        let _ = Plane::new(vec, 5.0);
        let _ = Plane::from_normal_at_origin(vec);
        let _ = Plane::from_point_normal(Vector3::new(10.0, 20.0, 30.0), vec);
        let _ = Plane::from_components(x, y, z, 5.0);
        let _ = Plane::from_points(
            Vector3::new(1.0, 2.0, 3.0),
            Vector3::new(2.0, 3.0, 1.0),
            Vector3::new(3.0, 2.0, 1.0),
        );
    }

    #[test]
    #[should_panic]
    fn new_unnormalized_panics() {
        let _ = Plane::new(Vector3::new(1.0, 2.0, 3.0), 5.0);
    }

    #[test]
    #[should_panic]
    fn from_points_colinear_panics() {
        let _ = Plane::from_points(Vector3::ZERO, Vector3::BACK, Vector3::new(0.0, 0.0, 2.0));
    }

    /// Tests `distance_to()`, `center()`, `contains_point()`, and `is_point_over()`.
    #[test]
    fn test_spatial_relations() {
        // Random plane that passes the origin point.
        let origin_plane = Plane::new(Vector3::new(1.0, 2.0, 3.0).normalized(), 0.0);

        // Parallels `origin_plane`.
        let parallel_origin_high = Plane::new(Vector3::new(1.0, 2.0, 3.0).normalized(), 1.0);
        let parallel_origin_low = Plane::new(Vector3::new(1.0, 2.0, 3.0).normalized(), -6.5);

        // Unrelated plane.
        let unrelated = Plane::new(Vector3::new(-1.0, 6.0, -5.0).normalized(), 3.2);

        // Origin point and center of `origin_plane`.
        let zero = Vector3::ZERO;
        assert_eq!(origin_plane.center(), zero);

        // Center of `parallel_origin_high`.
        let center_origin_high = parallel_origin_high.center();

        // Center of `parallel_origin_low`.
        let center_origin_low = parallel_origin_low.center();

        // The origin point should be in `origin_plane`, so results in 0.0 distance.
        assert!(origin_plane.contains_point(zero, None));
        assert_eq!(origin_plane.distance_to(zero), 0.0);

        // No matter the normals, the absolute distance to the origin point should always be the absolute
        // value of the plane's `d`.
        assert_eq!(origin_plane.distance_to(zero).abs(), origin_plane.d.abs());
        assert_eq!(
            parallel_origin_high.distance_to(zero).abs(),
            parallel_origin_high.d.abs()
        );
        assert_eq!(
            parallel_origin_low.distance_to(zero).abs(),
            parallel_origin_low.d.abs()
        );
        assert_eq!(unrelated.distance_to(zero).abs(), unrelated.d.abs());

        // The absolute distance between a plane and its parallel's center should always be the difference
        // between both the `d`s.
        assert!(parallel_origin_high.contains_point(center_origin_high, None));
        assert_eq_approx!(
            origin_plane.distance_to(center_origin_high).abs(),
            (origin_plane.d - parallel_origin_high.d).abs(),
        );
        assert!(parallel_origin_low.contains_point(center_origin_low, None));
        assert_eq_approx!(
            origin_plane.distance_to(center_origin_low).abs(),
            (origin_plane.d - parallel_origin_low.d).abs(),
        );

        // As `parallel_origin_high` is higher than `origin_plane` by having larger `d` value, then its center should be
        // higher than `origin_plane`.
        assert!(origin_plane.is_point_over(center_origin_high));

        // As `parallel_origin_low` is lower than `origin_plane` by having smaller `d` value, then its center should be
        // lower than `origin_plane`.
        assert!(!origin_plane.is_point_over(center_origin_low));

        // By the reasonings stated above, then the following should be correct.
        assert!(!parallel_origin_high.is_point_over(zero));
        assert!(!parallel_origin_high.is_point_over(center_origin_low));
        assert!(parallel_origin_low.is_point_over(zero));
        assert!(parallel_origin_low.is_point_over(center_origin_high));
    }

    /// Tests `intersect_3()`.
    #[test]
    fn test_three_planes_intersections() {
        // Planes that intersects in (0.0, 0.0, 0.0).
        let origin_plane_a = Plane::new(Vector3::new(1.0, 2.0, 0.0).normalized(), 0.0);
        let origin_plane_b = Plane::new(Vector3::new(3.5, 6.0, -3.0).normalized(), 0.0);
        let origin_plane_c = Plane::new(Vector3::new(-1.0, 6.0, 0.5).normalized(), 0.0);

        // Planes that parallels `origin_plane_a`.
        let low_parallel_origin_a = Plane::new(Vector3::new(1.0, 2.0, 0.0).normalized(), 1.0);
        let high_parallel_origin_a = Plane::new(Vector3::new(1.0, 2.0, 0.0).normalized(), 2.0);

        // Planes that intersects `origin_plane_a` and each other in a common line.
        let small_rotation_origin_a =
            Plane::new(origin_plane_a.normal.rotated(Vector3::BACK, 30.0), 0.0);
        let large_rotation_origin_a =
            Plane::new(origin_plane_a.normal.rotated(Vector3::BACK, 60.0), 0.0);

        // Planes that intersects each other in 3 parallel lines.
        let prism_plane_a = Plane::new(Vector3::new(2.5, -6.0, 0.0).normalized(), 1.0);
        let prism_plane_b = Plane::new(prism_plane_a.normal.rotated(Vector3::BACK, 30.0), 1.0);
        let prism_plane_c = Plane::new(prism_plane_a.normal.rotated(Vector3::BACK, 60.0), 1.0);

        // Origin point.
        let vec_a = Vector3::ZERO;

        // Planes that have 0 as its `d` would intersect in the origin point.
        assert_eq!(
            origin_plane_a.intersect_3(&origin_plane_b, &origin_plane_c),
            Some(vec_a)
        );

        // Three planes that parallel each other would not intersect in a point.
        assert_eq!(
            origin_plane_a.intersect_3(&low_parallel_origin_a, &high_parallel_origin_a),
            None
        );

        // Two planes that parallel each other with an unrelated third plane would not intersect in
        // a point.
        assert_eq!(
            origin_plane_b.intersect_3(&low_parallel_origin_a, &high_parallel_origin_a),
            None
        );

        // Three coincident planes would intersect in every point, thus no unique solution.
        assert_eq!(
            origin_plane_a.intersect_3(&origin_plane_a, &origin_plane_a),
            None
        );

        // Two coincident planes with an unrelated third plane would intersect in every point along the
        // intersection line, thus no unique solution.
        assert_eq!(
            origin_plane_b.intersect_3(&origin_plane_b, &large_rotation_origin_a),
            None
        );

        // Two coincident planes with a parallel third plane would have no common intersection.
        assert_eq!(
            origin_plane_a.intersect_3(&origin_plane_a, &low_parallel_origin_a),
            None
        );

        // Three planes that intersects each other in a common line would intersect in every point along
        // the line, thus no unique solution.
        assert_eq!(
            origin_plane_a.intersect_3(&small_rotation_origin_a, &large_rotation_origin_a),
            None
        );

        // Three planes that intersects each other in 3 parallel lines would not intersect in a common
        // point.
        assert_eq!(
            prism_plane_a.intersect_3(&prism_plane_b, &prism_plane_c),
            None
        );
    }

    /// Tests `intersect_ray()`.
    #[test]
    fn test_ray_intersections() {
        // Plane that is flat along the z-axis.
        let xy_plane = Plane::new(Vector3::BACK, 0.0);

        // Origin point.
        let zero = Vector3::ZERO;

        // Forms a straight line along the z-axis with `zero` that is perpendicular to plane.
        let low_pos_z = Vector3::new(0.0, 0.0, 0.5);
        let high_pos_z = Vector3::BACK;
        let neg_z = Vector3::FORWARD;

        // Forms a slanted line with `zero` relative to plane.
        let pos_xy = Vector3::new(0.5, 0.5, 0.0);

        // Forms a line with `high_pos_z` that is parallel with plane.
        let pos_xz = Vector3::new(1.0, 0.0, 1.0);

        // From a point straight up from the origin point, a ray pointing straight down would cross
        // the plane in the origin point.
        assert_eq!(xy_plane.intersect_ray(low_pos_z, neg_z), Some(zero));
        assert_eq!(xy_plane.intersect_ray(high_pos_z, neg_z), Some(zero));

        // From a point straight down the origin point, a ray pointing straight up would cross the plane
        // in the origin point.
        assert_eq!(xy_plane.intersect_ray(neg_z, low_pos_z), Some(zero));
        assert_eq!(xy_plane.intersect_ray(neg_z, high_pos_z), Some(zero));

        // A ray parallel to the plane would not intersect the plane.
        assert_eq!(xy_plane.intersect_ray(high_pos_z, pos_xz), None);

        // A ray pointing to the opposite direction as the plane would not intersect it.
        assert_eq!(xy_plane.intersect_ray(low_pos_z, high_pos_z), None);
        assert_eq!(xy_plane.intersect_ray(low_pos_z, pos_xy), None);
    }

    /// Tests `intersect_segment()`.
    #[test]
    fn test_segment_intersections() {
        // Plane that is flat along the z-axis.
        let xy_plane = Plane::new(Vector3::BACK, 0.0);

        // Origin point.
        let zero = Vector3::ZERO;

        // Forms a straight line along the z-axis with `zero` that is perpendicular to plane.
        let low_pos_z = Vector3::new(0.0, 0.0, 0.5);
        let high_pos_z = Vector3::BACK;
        let low_neg_z = Vector3::FORWARD;
        let high_neg_z = Vector3::new(0.0, 0.0, -0.5);

        // Forms a line with `high_pos_z` that is parallel with plane.
        let pos_xz = Vector3::new(1.0, 0.0, 1.0);

        // From a point straight up from the origin point, a segment pointing straight down would cross
        // the plane in the origin point only if the segment ended on or beyond the plane.
        assert_eq!(xy_plane.intersect_segment(low_pos_z, low_neg_z), Some(zero));
        assert_eq!(
            xy_plane.intersect_segment(high_pos_z, low_neg_z),
            Some(zero)
        );
        assert_eq!(xy_plane.intersect_segment(low_pos_z, zero), Some(zero));
        assert_eq!(xy_plane.intersect_segment(high_pos_z, zero), Some(zero));
        assert_eq!(xy_plane.intersect_segment(high_pos_z, low_pos_z), None);

        // From a point straight down the origin point, a segment pointing straight up would cross the plane
        // in the origin point only if the segment ended on or beyond the plane.
        assert_eq!(xy_plane.intersect_segment(low_neg_z, zero), Some(zero));
        assert_eq!(xy_plane.intersect_segment(low_neg_z, low_pos_z), Some(zero));
        assert_eq!(
            xy_plane.intersect_segment(low_neg_z, high_pos_z),
            Some(zero)
        );
        assert_eq!(xy_plane.intersect_segment(low_neg_z, high_neg_z), None);

        // A segment parallel to the plane would not intersect the plane.
        assert_eq!(xy_plane.intersect_segment(high_pos_z, pos_xz), None);

        // A segment pointing to the opposite direction as the plane would not intersect it.
        assert_eq!(xy_plane.intersect_segment(low_pos_z, high_pos_z), None);
        assert_eq!(xy_plane.intersect_segment(low_pos_z, pos_xz), None);
    }

    /// Tests `is_equal_approx()`.
    #[test]
    fn test_equal() {
        // Initial planes.
        let xy_plane = Plane::new(Vector3::BACK, 0.0);
        let almost_xy_plane_a = Plane::new(Vector3::new(0.01, 0.0, 1.0).normalized(), 0.0);
        let almost_xy_plane_b = Plane::new(Vector3::new(0.0001, 0.0, 1.0).normalized(), 0.0);
        let almost_xy_plane_c = Plane::new(Vector3::new(0.000001, 0.0, 1.0).normalized(), 0.0);
        let approx_xy_plane_a = Plane::new(Vector3::new(0.000001, 0.0, 1.0).normalized(), 0.01);
        let approx_xy_plane_b = Plane::new(Vector3::new(0.000001, 0.0, 1.0).normalized(), 0.000001);

        // Same planes should be equals.
        assert_eq_approx!(xy_plane, xy_plane);
        assert_eq_approx!(almost_xy_plane_a, almost_xy_plane_a);

        // Planes below should be approximately equal because it's lower than the set tolerance constant.
        assert_eq_approx!(xy_plane, almost_xy_plane_c);

        // Both attributes are approximately equal.
        assert_eq_approx!(xy_plane, approx_xy_plane_b);

        // Although similar, planes below are not approximately equals.
        assert_ne_approx!(xy_plane, almost_xy_plane_a);
        assert_ne_approx!(xy_plane, almost_xy_plane_b);

        // Although approximately equal in the `normal` part, it is not approximately equal in the `d`
        // part.
        assert_ne_approx!(xy_plane, approx_xy_plane_a);

        // Although considered approximately equal with `xy_plane`, `almost_xy_plane_a` is not considered approximately
        // equal with `almost_xy_plane_d` because the baseline comparison is tighter.
        assert_ne_approx!(almost_xy_plane_a, approx_xy_plane_a);
    }

    /// Tests `normalize()`.
    #[test]
    fn test_normalization() {
        // Non-normalized planes.
        let plane = Plane {
            normal: Vector3::new(0.7, 2.0, 6.0),
            d: 0.0,
        };
        assert_eq!(plane.normalized().normal, plane.normal.normalized());

        let plane = Plane {
            normal: Vector3::new(1.5, 7.2, 9.1),
            d: 2.0,
        };
        assert_eq!(plane.normalized().normal, plane.normal.normalized());

        let plane = Plane {
            normal: Vector3::new(1.4, 9.1, 1.2),
            d: 5.3,
        };
        assert_eq!(plane.normalized().normal, plane.normal.normalized());
        let plane = Plane {
            normal: Vector3::new(4.2, 2.9, 1.5),
            d: 2.4,
        };
        assert_eq!(plane.normalized().normal, plane.normal.normalized());

        // Normalized plane.
        let plane = Plane {
            normal: Vector3::new(5.1, 3.0, 2.1).normalized(),
            d: 0.2,
        };
        assert_eq!(plane.normalized().normal, plane.normal.normalized());
    }

    /// Tests `is_finite()`.
    #[test]
    fn test_finite() {
        // Non-finite planes.
        let plane = Plane {
            normal: Vector3::new(0.7, real::INFINITY, -6.0),
            d: 10.2,
        };
        assert!(!plane.is_finite());

        let plane = Plane {
            normal: Vector3::new(0.7, 2.0, real::NEG_INFINITY),
            d: 10.2,
        };
        assert!(!plane.is_finite());

        let plane = Plane {
            normal: Vector3::new(0.7, real::INFINITY, -6.0),
            d: real::INFINITY,
        };
        assert!(!plane.is_finite());

        let plane = Plane {
            normal: Vector3::new(real::NAN, real::INFINITY, real::NEG_INFINITY),
            d: real::NAN,
        };
        assert!(!plane.is_finite());

        // Finite plane.
        let plane = Plane {
            normal: Vector3::new(7.2, -2.9, 2.2).normalized(),
            d: 3.3,
        };
        assert!(plane.is_finite());
    }

    /// Tests `project()` and `center()`.
    #[test]
    fn test_projection() {
        // Plane that is flat along the z-axis.
        let xy_plane = Plane::new(Vector3::BACK, 0.0);

        // Parallels `xy_plane`
        let parallel_xy_plane = Plane::new(Vector3::BACK, 3.5);

        // Random vectors.
        let random_a = Vector3::new(0.0, 3.2, 1.5);
        let random_b = Vector3::new(1.1, 7.3, -6.4);
        let random_c = Vector3::new(0.5, -7.2, 0.2);

        // Projection of points to `xy_plane` would result in the same points, with the z aspect of the
        // vector be 0.0.
        assert_eq!(
            xy_plane.project(random_a),
            Vector3::new(random_a.x, random_a.y, 0.0)
        );
        assert_eq!(
            xy_plane.project(random_b),
            Vector3::new(random_b.x, random_b.y, 0.0)
        );
        assert_eq!(
            xy_plane.project(random_c),
            Vector3::new(random_c.x, random_c.y, 0.0)
        );

        // Projection of the center of a plane that parallels the plane that is being projected into
        // is going to be the center of the plane that is being projected.
        assert_eq!(
            xy_plane.project(parallel_xy_plane.center()),
            xy_plane.center()
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_roundtrip() {
        let plane = Plane {
            normal: Vector3::ONE,
            d: 0.0,
        };
        let expected_json = "{\"normal\":{\"x\":1.0,\"y\":1.0,\"z\":1.0},\"d\":0.0}";

        crate::builtin::test_utils::roundtrip(&plane, expected_json);
    }
}
