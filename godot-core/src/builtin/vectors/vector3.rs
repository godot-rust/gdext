/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::cmp::Ordering;
use std::fmt;

use godot_ffi as sys;
use sys::{ffi_methods, ExtVariantType, GodotFfi};

use crate::builtin::math::{FloatExt, GlamConv, GlamType};
use crate::builtin::vectors::Vector3Axis;
use crate::builtin::{inner, real, Basis, RVec3, Vector2, Vector3i};

/// Vector used for 3D math using floating point coordinates.
///
/// 3-element structure that can be used to represent continuous positions or directions in 3D space,
/// as well as any other triple of numeric values.
///
/// It uses floating-point coordinates of 32-bit precision, unlike the engine's `float` type which
/// is always 64-bit. The engine can be compiled with the option `precision=double` to use 64-bit
/// vectors instead; use the gdext library with the `double-precision` feature in that case.
///
#[doc = shared_vector_docs!()]
///
/// ### Navigation to `impl` blocks within this page
///
/// - [Constants](#constants)
/// - [Constructors and general vector functions](#constructors-and-general-vector-functions)
/// - [Specialized `Vector3` functions](#specialized-vector3-functions)
/// - [Float-specific functions](#float-specific-functions)
/// - [3D functions](#3d-functions)
/// - [2D and 3D functions](#2d-and-3d-functions)
/// - [3D and 4D functions](#3d-and-4d-functions)
/// - [Trait impls + operators](#trait-implementations)
///
/// # All vector types
///
/// | Dimension | Floating-point                       | Integer                                |
/// |-----------|--------------------------------------|----------------------------------------|
/// | 2D        | [`Vector2`][crate::builtin::Vector2] | [`Vector2i`][crate::builtin::Vector2i] |
/// | 3D        | **`Vector3`**                        | [`Vector3i`][crate::builtin::Vector3i] |
/// | 4D        | [`Vector4`][crate::builtin::Vector4] | [`Vector4i`][crate::builtin::Vector4i] |
///
/// <br>You can convert to `Vector3i` using [`cast_int()`][Self::cast_int].
///
/// # Godot docs
///
/// [`Vector3` (stable)](https://docs.godotengine.org/en/stable/classes/class_vector3.html)
#[derive(Default, Copy, Clone, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(C)]
pub struct Vector3 {
    /// The vector's X component.
    pub x: real,

    /// The vector's Y component.
    pub y: real,

    /// The vector's Z component.
    pub z: real,
}

/// # Constants
impl Vector3 {
    impl_vector_consts!(real);
    impl_float_vector_consts!();
    impl_vector3x_consts!(real);

    /// Unit vector pointing towards the left side of imported 3D assets.
    pub const MODEL_LEFT: Self = Self::new(1.0, 0.0, 0.0);

    /// Unit vector pointing towards the right side of imported 3D assets.
    pub const MODEL_RIGHT: Self = Self::new(-1.0, 0.0, 0.0);

    /// Unit vector pointing towards the top side (up) of imported 3D assets.
    pub const MODEL_TOP: Self = Self::new(0.0, 1.0, 0.0);

    /// Unit vector pointing towards the bottom side (down) of imported 3D assets.
    pub const MODEL_BOTTOM: Self = Self::new(0.0, -1.0, 0.0);

    /// Unit vector pointing towards the front side (facing forward) of imported 3D assets.
    pub const MODEL_FRONT: Self = Self::new(0.0, 0.0, 1.0);

    /// Unit vector pointing towards the rear side (back) of imported 3D assets.
    pub const MODEL_REAR: Self = Self::new(0.0, 0.0, -1.0);
}

impl_vector_fns!(Vector3, RVec3, real, (x, y, z));

/// # Specialized `Vector3` functions
impl Vector3 {
    #[doc(hidden)]
    #[inline]
    pub fn as_inner(&self) -> inner::InnerVector3<'_> {
        inner::InnerVector3::from_outer(self)
    }

    /// Returns the cross product of this vector and `with`.
    ///
    /// This returns a vector perpendicular to both this and `with`, which would be the normal vector of the plane
    /// defined by the two vectors. As there are two such vectors, in opposite directions,
    /// this method returns the vector defined by a right-handed coordinate system.
    /// If the two vectors are parallel this returns an empty vector, making it useful for testing if two vectors are parallel.
    #[inline]
    pub fn cross(self, with: Self) -> Self {
        Self::from_glam(self.to_glam().cross(with.to_glam()))
    }

    /// Returns the Vector3 from an octahedral-compressed form created using [`Vector3::octahedron_encode`] (stored as a [`Vector2`]).
    #[inline]
    pub fn octahedron_decode(uv: Vector2) -> Self {
        let f = Vector2::new(uv.x * 2.0 - 1.0, uv.y * 2.0 - 1.0);
        let mut n = Vector3::new(f.x, f.y, 1.0 - f.x.abs() - f.y.abs());

        let t = (-n.z).clamp(0.0, 1.0);
        n.x += if n.x >= 0.0 { -t } else { t };
        n.y += if n.y >= 0.0 { -t } else { t };

        n.normalized()
    }

    /// Returns the octahedral-encoded (oct32) form of this `Vector3` as a [`Vector2`].
    ///
    /// Since a [`Vector2`] occupies 1/3 less memory compared to `Vector3`, this form of compression can be used to pass greater amounts of
    /// [`Vector3::normalized`] `Vector3`s without increasing storage or memory requirements. See also [`Vector3::octahedron_decode`].
    ///
    /// Note: Octahedral compression is lossy, although visual differences are rarely perceptible in real world scenarios.
    ///
    /// # Panics
    /// If vector is not normalized.
    #[inline]
    pub fn octahedron_encode(self) -> Vector2 {
        assert!(self.is_normalized(), "vector is not normalized!");

        let mut n = self;
        n /= n.x.abs() + n.y.abs() + n.z.abs();

        let mut o = if n.z >= 0.0 {
            Vector2::new(n.x, n.y)
        } else {
            let x = (1.0 - n.y.abs()) * (if n.x >= 0.0 { 1.0 } else { -1.0 });
            let y = (1.0 - n.x.abs()) * (if n.y >= 0.0 { 1.0 } else { -1.0 });

            Vector2::new(x, y)
        };

        o.x = o.x * 0.5 + 0.5;
        o.y = o.y * 0.5 + 0.5;

        o
    }

    /// Returns the outer product with `with`.
    #[inline]
    pub fn outer(self, with: Self) -> Basis {
        let x = Vector3::new(self.x * with.x, self.x * with.y, self.x * with.z);
        let y = Vector3::new(self.y * with.x, self.y * with.y, self.y * with.z);
        let z = Vector3::new(self.z * with.x, self.z * with.y, self.z * with.z);

        Basis::from_rows(x, y, z)
    }

    /// Returns this vector rotated around `axis` by `angle` radians. `axis` must be normalized.
    ///
    /// # Panics
    /// If `axis` is not normalized.
    #[inline]
    pub fn rotated(self, axis: Self, angle: real) -> Self {
        assert!(axis.is_normalized(), "axis is not normalized!");
        Basis::from_axis_angle(axis, angle) * self
    }

    /// Returns the **unsigned** angle between `self` and the given vector, as radians in `[0, +π]`.
    ///
    /// Note that behavior is different from 2D [`Vector2::angle_to()`], which returns the **signed** angle.
    #[inline]
    pub fn angle_to(self, to: Self) -> real {
        self.glam2(&to, |a, b| a.angle_between(b))
    }

    /// Returns the signed angle to the given vector, as radians in `[-π, +π]`.
    ///
    /// The sign of the angle is positive in a counter-clockwise direction and negative in a clockwise direction, when viewed from
    /// the side specified by the `axis`.
    ///
    /// For unsigned angles, use [`Vector3::angle_to()`].
    #[inline]
    pub fn signed_angle_to(self, to: Self, axis: Self) -> real {
        let cross_to = self.cross(to);
        let unsigned_angle = cross_to.length().atan2(self.dot(to));
        let sign = cross_to.dot(axis);
        if sign < 0.0 {
            -unsigned_angle
        } else {
            unsigned_angle
        }
    }

    /// Returns the spherical linear interpolation between the vector and `to` by the `weight` amount.
    ///
    /// The variable `weight` is representing the amount of interpolation, which is on the range of
    /// 0.0 to 1.0.
    ///
    /// Length is also interpolated in the case that the input vectors have different lengths. If both
    /// input vectors have zero length or are collinear to each other, the method instead behaves like
    /// [`Vector3::lerp`].
    #[inline]
    pub fn slerp(self, to: Self, weight: real) -> Self {
        let start_length_sq: real = self.length_squared();
        let end_length_sq = to.length_squared();
        if start_length_sq == 0.0 || end_length_sq == 0.0 {
            // Vectors with zero length do not have an angle relative to the origin point, so it cannot
            // produce a cross product for determining the angle to slerp into. Because of this, lerp
            // is used to interpolate between the two vectors.
            return self.lerp(to, weight);
        }

        let axis = self.cross(to);
        if axis == Vector3::ZERO {
            // Two collinear vectors do not have a unique perpendicular axis to both of them, so it
            // cannot produce a cross product for determining the angle to slerp into. Because of this,
            // lerp is used to interpolate between the two vectors.
            return self.lerp(to, weight);
        }

        let unit_axis = axis.normalized();
        let start_length = start_length_sq.sqrt();
        let result_length = start_length.lerp(end_length_sq.sqrt(), weight);
        let angle = self.angle_to(to);
        self.rotated(unit_axis, angle * weight) * (result_length / start_length)
    }
}

impl_float_vector_fns!(Vector3, Vector3i, (x, y, z));
impl_vector3x_fns!(Vector3, Vector2, real);
impl_vector2_vector3_fns!(Vector3, (x, y, z));
impl_vector3_vector4_fns!(Vector3, (x, y, z));

impl_vector_operators!(Vector3, real, (x, y, z));

/// Formats the vector like Godot: `(x, y, z)`.
impl fmt::Display for Vector3 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {}, {})", self.x, self.y, self.z)
    }
}

// SAFETY:
// This type is represented as `Self` in Godot, so `*mut Self` is sound.
unsafe impl GodotFfi for Vector3 {
    const VARIANT_TYPE: ExtVariantType = ExtVariantType::Concrete(sys::VariantType::VECTOR3);

    ffi_methods! { type sys::GDExtensionTypePtr = *mut Self; .. }
}

crate::meta::impl_godot_as_self!(Vector3: ByValue);

impl GlamType for RVec3 {
    type Mapped = Vector3;

    fn to_front(&self) -> Self::Mapped {
        Vector3::new(self.x, self.y, self.z)
    }

    fn from_front(mapped: &Self::Mapped) -> Self {
        RVec3::new(mapped.x, mapped.y, mapped.z)
    }
}

#[cfg(not(feature = "double-precision"))]
impl GlamType for glam::Vec3A {
    type Mapped = Vector3;

    fn to_front(&self) -> Self::Mapped {
        Vector3::new(self.x, self.y, self.z)
    }

    fn from_front(mapped: &Self::Mapped) -> Self {
        glam::Vec3A::new(mapped.x, mapped.y, mapped.z)
    }
}

impl GlamConv for Vector3 {
    type Glam = RVec3;
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::builtin::math::assert_eq_approx;
    use crate::builtin::real_consts::{SQRT_2, TAU};

    // Translated from Godot
    #[test]
    #[allow(clippy::excessive_precision)]
    fn rotation() {
        let vector = Vector3::new(1.2, 3.4, 5.6);
        assert_eq_approx!(
            vector.rotated(Vector3::new(0.0, 1.0, 0.0), TAU), //.
            vector
        );
        assert_eq_approx!(
            vector.rotated(Vector3::new(0.0, 1.0, 0.0), TAU / 4.0),
            Vector3::new(5.6, 3.4, -1.2),
        );
        assert_eq_approx!(
            vector.rotated(Vector3::new(1.0, 0.0, 0.0), TAU / 3.0),
            Vector3::new(1.2, -6.54974226119285642, 0.1444863728670914),
        );
        assert_eq_approx!(
            vector.rotated(Vector3::new(0.0, 0.0, 1.0), TAU / 2.0),
            vector.rotated(Vector3::new(0.0, 0.0, 1.0), TAU / -2.0),
        );
    }

    #[test]
    fn sign() {
        let vector = Vector3::new(0.2, -0.5, 0.0);
        assert_eq!(vector.sign(), Vector3::new(1., -1., 0.));
    }

    #[test]
    fn coord_min_max() {
        let a = Vector3::new(1.2, 3.4, 5.6);
        let b = Vector3::new(0.1, 5.6, 2.3);

        assert_eq_approx!(a.coord_min(b), Vector3::new(0.1, 3.4, 2.3));
        assert_eq_approx!(a.coord_max(b), Vector3::new(1.2, 5.6, 5.6));
    }

    #[test]
    fn test_slerp() {
        // The halfway point of a slerp operation on two vectors on a circle is the halfway point of
        // the arc length between the two vectors.
        let vector_from = Vector3::new(0.0, 2.0, 0.0);
        let vector_to = Vector3::new(2.0, 0.0, 0.0);
        let vector_in_between = Vector3::new(SQRT_2, SQRT_2, 0.0);
        assert_eq_approx!(vector_from.slerp(vector_to, 0.5), vector_in_between);

        // Collinear vectors cannot be slerped so the halfway point of the slerp operation on them is
        // just the halfway point between them.
        let vector_from = Vector3::new(0.0, 2.0, 0.0);
        let vector_to = Vector3::new(0.0, -2.0, 0.0);
        assert_eq_approx!(vector_from.slerp(vector_to, 0.5), Vector3::ZERO);

        let vector_from = Vector3::new(0.0, 3.0, 0.0);
        let vector_to = Vector3::new(0.0, 2.0, 0.0);
        assert_eq_approx!(
            vector_from.slerp(vector_to, 0.5),
            Vector3::new(0.0, 2.5, 0.0)
        );

        // Ported Godot slerp tests.
        let vector1 = Vector3::new(1.0, 2.0, 3.0);
        let vector2 = Vector3::new(4.0, 5.0, 6.0);
        assert_eq_approx!(
            vector1.normalized().slerp(vector2.normalized(), 0.5),
            Vector3::new(0.363_866_8, 0.555_698_2, 0.747_529_57)
        );
        assert_eq_approx!(
            vector1.normalized().slerp(vector2.normalized(), 1.0 / 3.0),
            Vector3::new(0.332_119_76, 0.549_413_74, 0.766_707_84)
        );
        assert_eq_approx!(
            Vector3::new(5.0, 0.0, 0.0).slerp(Vector3::new(0.0, 3.0, 4.0), 0.5),
            Vector3::new(3.535_534, 2.121_320_5, 2.828_427_3)
        );
        assert_eq_approx!(
            Vector3::new(1.0, 1.0, 1.0).slerp(Vector3::new(2.0, 2.0, 2.0), 0.5),
            Vector3::new(1.5, 1.5, 1.5)
        );
        assert_eq!(Vector3::ZERO.slerp(Vector3::ZERO, 0.5), Vector3::ZERO);
        assert_eq!(
            Vector3::ZERO.slerp(Vector3::new(1.0, 1.0, 1.0), 0.5),
            Vector3::new(0.5, 0.5, 0.5)
        );
        assert_eq!(
            Vector3::new(1.0, 1.0, 1.0).slerp(Vector3::ZERO, 0.5),
            Vector3::new(0.5, 0.5, 0.5)
        );
        assert_eq_approx!(
            Vector3::new(4.0, 6.0, 2.0).slerp(Vector3::new(8.0, 10.0, 3.0), 0.5),
            Vector3::new(5.901_942_3, 8.067_587, 2.558_308)
        );
        assert_eq_approx!(vector1.slerp(vector2, 0.5).length(), real!(6.258_311));
    }

    #[test]
    fn iter_sum() {
        let vecs = vec![
            Vector3::new(1.0, 2.0, 3.0),
            Vector3::new(4.0, 5.0, 6.0),
            Vector3::new(7.0, 8.0, 9.0),
        ];

        let sum_refs = vecs.iter().sum();
        let sum = vecs.into_iter().sum();

        assert_eq_approx!(sum, Vector3::new(12.0, 15.0, 18.0));
        assert_eq_approx!(sum_refs, Vector3::new(12.0, 15.0, 18.0));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_roundtrip() {
        let vector = Vector3::default();
        let expected_json = "{\"x\":0.0,\"y\":0.0,\"z\":0.0}";

        crate::builtin::test_utils::roundtrip(&vector, expected_json);
    }
}
