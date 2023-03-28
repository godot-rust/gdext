/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::ops::Neg;

use godot_ffi as sys;
use sys::{ffi_methods, GodotFfi};

use super::{is_equal_approx, real, Vector3};

/// 3D plane in [Hessian normal form](https://mathworld.wolfram.com/HessianNormalForm.html).
///
/// The Hessian form defines all points `point` which satisfy the equation
/// `dot(normal, point) + d == 0`, where `normal` is the normal vector and `d`
/// the distance from the origin.
///
/// Currently most methods are only available through [`InnerPlane`](super::inner::InnerPlane).
///
/// Note: almost all methods on `Plane` require that the `normal` vector have
/// unit length and will panic if this invariant is violated. This is not separately
/// annotated for each method.
#[derive(Copy, Clone, PartialEq, Debug)]
#[repr(C)]
pub struct Plane {
    pub normal: Vector3,
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

    /// Returns `true` if the two `Plane`s are approximately equal, by calling `is_equal_approx` on
    /// `normal` and `d` or on `-normal` and `-d`.
    ///
    /// _Godot equivalent: `Plane.is_equal_approx()`_
    #[inline]
    pub fn is_equal_approx(&self, other: &Self) -> bool {
        (self.normal.is_equal_approx(other.normal) && is_equal_approx(self.d, other.d))
            || (self.normal.is_equal_approx(-other.normal) && is_equal_approx(self.d, -other.d))
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

impl GodotFfi for Plane {
    ffi_methods! { type sys::GDExtensionTypePtr = *mut Self; .. }
}

#[cfg(test)]
mod test {
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
        let _ = Plane::from_points(
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(0.0, 0.0, 1.0),
            Vector3::new(0.0, 0.0, 2.0),
        );
    }
}
