/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::fmt::Display;
use std::ops::Mul;

use godot_ffi as sys;
use sys::{ffi_methods, ExtVariantType, GodotFfi};

use crate::builtin::math::{ApproxEq, GlamConv, GlamType, XformInv};
use crate::builtin::{real, Aabb, Basis, Plane, Projection, RAffine3, Vector3};

/// Affine 3D transform (3x4 matrix).
///
/// Used for 3D linear transformations. Uses a basis + origin representation.
///
/// Expressed as a 3x4 matrix, this transform consists of 3 basis (column)
/// vectors `a`, `b`, `c` as well as an origin `o`:
/// ```text
/// [ a.x  b.x  c.x  o.x ]
/// [ a.y  b.y  c.y  o.y ]
/// [ a.z  b.z  c.z  o.z ]
/// ```
///
/// # All matrix types
///
/// | Dimension | Orthogonal basis | Affine transform        | Projective transform |
/// |-----------|------------------|-------------------------|----------------------|
/// | 2D        |                  | [`Transform2D`] (2x3)   |                      |
/// | 3D        | [`Basis`] (3x3)  | **`Transform3D`** (3x4) | [`Projection`] (4x4) |
///
/// [`Basis`]: Basis
/// [`Transform2D`]: crate::builtin::Transform2D
/// [`Projection`]: Projection
///
/// # Transform operations
///
/// | Operation                      | Transform3D                    | Notes                                      |
/// |--------------------------------|--------------------------------|--------------------------------------------|
/// | Apply                          | `transform * v`                | Supports [`Aabb`], [`Plane`], [`Vector3`]. |
/// | Apply inverse                  | `transform.xform_inv(v)`       | Supports [`Aabb`], [`Plane`], [`Vector3`]. |
/// | Apply, no translate            | `transform.basis * v`          | Supports [`Vector3`].                      |
/// | Apply inverse, no translate    | `transform.basis.xform_inv(v)` | Supports [`Vector3`].                      |
///
/// # Godot docs
///
/// [`Transform3D` (stable)](https://docs.godotengine.org/en/stable/classes/class_transform3d.html)
#[derive(Default, Copy, Clone, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(C)]
pub struct Transform3D {
    /// The basis is a matrix containing 3 vectors as its columns. They can be
    /// interpreted as the basis vectors of the transformed coordinate system.
    pub basis: Basis,

    /// The new origin of the transformed coordinate system.
    pub origin: Vector3,
}

impl Transform3D {
    /// The identity transform, with no translation, rotation or scaling
    /// applied. When applied to other data structures, `IDENTITY` performs no
    /// transformation.
    ///
    /// _Godot equivalent: `Transform3D.IDENTITY`_
    pub const IDENTITY: Self = Self::new(Basis::IDENTITY, Vector3::ZERO);

    /// `Transform3D` with mirroring applied perpendicular to the YZ plane.
    ///
    /// _Godot equivalent: `Transform3D.FLIP_X`_
    pub const FLIP_X: Self = Self::new(Basis::FLIP_X, Vector3::ZERO);

    /// `Transform3D` with mirroring applied perpendicular to the XZ plane.
    ///
    /// _Godot equivalent: `Transform3D.FLIP_Y`_
    pub const FLIP_Y: Self = Self::new(Basis::FLIP_Y, Vector3::ZERO);

    /// `Transform3D` with mirroring applied perpendicular to the XY plane.
    ///
    /// _Godot equivalent: `Transform3D.FLIP_Z`_
    pub const FLIP_Z: Self = Self::new(Basis::FLIP_Z, Vector3::ZERO);

    /// Create a new transform from a [`Basis`] and a [`Vector3`].
    ///
    /// _Godot equivalent: `Transform3D(Basis basis, Vector3 origin)`_
    pub const fn new(basis: Basis, origin: Vector3) -> Self {
        Self { basis, origin }
    }

    /// Create a new transform from 4 matrix-columns.
    ///
    /// _Godot equivalent: `Transform3D(Vector3 x_axis, Vector3 y_axis, Vector3 z_axis, Vector3 origin)`_, see [`Basis`][crate::builtin::Basis]
    /// for why it's changed
    pub const fn from_cols(a: Vector3, b: Vector3, c: Vector3, origin: Vector3) -> Self {
        Self {
            basis: Basis::from_cols(a, b, c),
            origin,
        }
    }

    /// Constructs a `Transform3D` from a `Projection` by trimming the last row of the projection matrix.
    ///
    /// _Godot equivalent: `Transform3D(Projection from)`_
    pub fn from_projection(proj: &Projection) -> Self {
        let a = Vector3::new(proj.cols[0].x, proj.cols[0].y, proj.cols[0].z);
        let b = Vector3::new(proj.cols[1].x, proj.cols[1].y, proj.cols[1].z);
        let c = Vector3::new(proj.cols[2].x, proj.cols[2].y, proj.cols[2].z);
        let o = Vector3::new(proj.cols[3].x, proj.cols[3].y, proj.cols[3].z);

        Self {
            basis: Basis::from_cols(a, b, c),
            origin: o,
        }
    }

    /// Unstable, used to simplify codegen. Too many parameters for public API and easy to have off-by-one, `from_cols()` is preferred.
    #[doc(hidden)]
    #[rustfmt::skip]
    #[allow(clippy::too_many_arguments)]
    pub const fn __internal_codegen(
        ax: real, ay: real, az: real,
        bx: real, by: real, bz: real,
        cx: real, cy: real, cz: real,
        ox: real, oy: real, oz: real
    ) -> Self {
        Self::from_cols(
            Vector3::new(ax, ay, az),
            Vector3::new(bx, by, bz),
            Vector3::new(cx, cy, cz),
            Vector3::new(ox, oy, oz),
        )
    }

    /// Returns the inverse of the transform, under the assumption that the
    /// transformation is composed of rotation, scaling and translation.
    #[must_use]
    pub fn affine_inverse(&self) -> Self {
        self.glam(|aff| aff.inverse())
    }

    /// Returns a transform interpolated between this transform and another by
    /// a given weight (on the range of 0.0 to 1.0).
    #[must_use]
    pub fn interpolate_with(&self, other: &Self, weight: real) -> Self {
        let src_scale = self.basis.get_scale();
        let src_rot = self.basis.get_quaternion().normalized();
        let src_loc = self.origin;

        let dst_scale = other.basis.get_scale();
        let dst_rot = other.basis.get_quaternion().normalized();
        let dst_loc = other.origin;

        let mut basis = Basis::from_scale(src_scale.lerp(dst_scale, weight));
        basis = Basis::from_quaternion(src_rot.slerp(dst_rot, weight)) * basis;

        Self {
            basis,
            origin: src_loc.lerp(dst_loc, weight),
        }
    }

    /// Returns true if this transform is finite by calling `is_finite` on the
    /// basis and origin.
    pub fn is_finite(&self) -> bool {
        self.basis.is_finite() && self.origin.is_finite()
    }

    /// Returns the transform with the basis orthogonal (90 degrees), and
    /// normalized axis vectors (scale of 1 or -1).
    ///
    /// _Godot equivalent: Transform3D.orthonormalized()_
    #[must_use]
    pub fn orthonormalized(&self) -> Self {
        Self {
            basis: self.basis.orthonormalized(),
            origin: self.origin,
        }
    }

    /// Returns a copy of the transform rotated by the given `angle` (in radians).
    /// This method is an optimized version of multiplying the given transform `X`
    /// with a corresponding rotation transform `R` from the left, i.e., `R * X`.
    /// This can be seen as transforming with respect to the global/parent frame.
    ///
    /// _Godot equivalent: `Transform2D.rotated()`_
    #[must_use]
    pub fn rotated(&self, axis: Vector3, angle: real) -> Self {
        let rotation = Basis::from_axis_angle(axis, angle);
        Self {
            basis: rotation * self.basis,
            origin: rotation * self.origin,
        }
    }
    /// Returns a copy of the transform rotated by the given `angle` (in radians).
    /// This method is an optimized version of multiplying the given transform `X`
    /// with a corresponding rotation transform `R` from the right, i.e., `X * R`.
    /// This can be seen as transforming with respect to the local frame.
    ///
    /// _Godot equivalent: `Transform2D.rotated_local()`_
    #[must_use]
    pub fn rotated_local(&self, axis: Vector3, angle: real) -> Self {
        Self {
            basis: self.basis * Basis::from_axis_angle(axis, angle),
            origin: self.origin,
        }
    }

    /// Returns a copy of the transform scaled by the given scale factor.
    /// This method is an optimized version of multiplying the given transform `X`
    /// with a corresponding scaling transform `S` from the left, i.e., `S * X`.
    /// This can be seen as transforming with respect to the global/parent frame.
    ///
    /// _Godot equivalent: `Transform2D.scaled()`_
    #[must_use]
    pub fn scaled(&self, scale: Vector3) -> Self {
        Self {
            basis: Basis::from_scale(scale) * self.basis,
            origin: self.origin * scale,
        }
    }

    /// Returns a copy of the transform scaled by the given scale factor.
    /// This method is an optimized version of multiplying the given transform `X`
    /// with a corresponding scaling transform `S` from the right, i.e., `X * S`.
    /// This can be seen as transforming with respect to the local frame.
    ///
    /// _Godot equivalent: `Transform2D.scaled_local()`_
    #[must_use]
    pub fn scaled_local(&self, scale: Vector3) -> Self {
        Self {
            basis: self.basis * Basis::from_scale(scale),
            origin: self.origin,
        }
    }

    /// Returns a copy of the transform translated by the given offset.
    /// This method is an optimized version of multiplying the given transform `X`
    /// with a corresponding translation transform `T` from the left, i.e., `T * X`.
    /// This can be seen as transforming with respect to the global/parent frame.
    ///
    /// _Godot equivalent: `Transform2D.translated()`_
    #[must_use]
    pub fn translated(&self, offset: Vector3) -> Self {
        Self {
            basis: self.basis,
            origin: self.origin + offset,
        }
    }

    /// Returns a copy of the transform translated by the given offset.
    /// This method is an optimized version of multiplying the given transform `X`
    /// with a corresponding translation transform `T` from the right, i.e., `X * T`.
    /// This can be seen as transforming with respect to the local frame.
    ///
    /// _Godot equivalent: `Transform2D.translated()`_
    #[must_use]
    pub fn translated_local(&self, offset: Vector3) -> Self {
        Self {
            basis: self.basis,
            origin: self.origin + (self.basis * offset),
        }
    }
}

impl Display for Transform3D {
    /// Formats the value with the given formatter.  [Read more](https://doc.rust-lang.org/1.79.0/core/fmt/trait.Display.html#tymethod.fmt)
    ///
    /// The output is similar to Godot's, but calls the columns a/b/c instead of X/Y/Z.  See [`Basis`][crate::builtin::Basis] for why.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Godot output:
        // [X: (1, 2, 3), Y: (4, 5, 6), Z: (7, 8, 9), O: (10, 11, 12)]
        // Where X,Y,Z,O are the columns
        let [a, b, c] = self.basis.to_cols();
        let o = self.origin;

        write!(f, "[a: {a}, b: {b}, c: {c}, o: {o}]")
    }
}

impl From<Basis> for Transform3D {
    /// Create a new transform with origin `(0,0,0)` from this basis.
    fn from(basis: Basis) -> Self {
        Self::new(basis, Vector3::ZERO)
    }
}

impl Mul for Transform3D {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        self.glam2(&rhs, |a, b| a * b)
    }
}

impl Mul<Vector3> for Transform3D {
    type Output = Vector3;

    fn mul(self, rhs: Vector3) -> Self::Output {
        self.glam2(&rhs, |t, v| t.transform_point3(v))
    }
}

impl XformInv<Vector3> for Transform3D {
    /// Inversely transforms given [`Vector3`] by this transformation matrix,
    /// under the assumption that the transformation basis is orthonormal (i.e. rotation/reflection is fine, scaling/skew is not).
    ///
    /// For transforming by inverse of an affine transformation (e.g. with scaling) `transform.affine_inverse() * vector` can be used instead. See [`Transform3D::affine_inverse()`].
    ///
    /// _Godot equivalent: `aabb * transform`_
    fn xform_inv(&self, rhs: Vector3) -> Vector3 {
        let v = rhs - self.origin;
        self.basis.xform_inv(v)
    }
}

impl Mul<real> for Transform3D {
    type Output = Self;

    fn mul(self, rhs: real) -> Self::Output {
        Self {
            basis: self.basis * rhs,
            origin: self.origin * rhs,
        }
    }
}

impl Mul<Aabb> for Transform3D {
    type Output = Aabb;

    /// Transforms each coordinate in `rhs.position` and `rhs.end()` individually by this transform, then
    /// creates an `Aabb` containing all of them.
    fn mul(self, rhs: Aabb) -> Self::Output {
        // https://web.archive.org/web/20220317024830/https://dev.theomader.com/transform-bounding-boxes/
        let xa = self.basis.col_a() * rhs.position.x;
        let xb = self.basis.col_a() * rhs.end().x;

        let ya = self.basis.col_b() * rhs.position.y;
        let yb = self.basis.col_b() * rhs.end().y;

        let za = self.basis.col_c() * rhs.position.z;
        let zb = self.basis.col_c() * rhs.end().z;

        let position = Vector3::coord_min(xa, xb)
            + Vector3::coord_min(ya, yb)
            + Vector3::coord_min(za, zb)
            + self.origin;
        let end = Vector3::coord_max(xa, xb)
            + Vector3::coord_max(ya, yb)
            + Vector3::coord_max(za, zb)
            + self.origin;
        Aabb::new(position, end - position)
    }
}

impl XformInv<Aabb> for Transform3D {
    /// Inversely transforms each vertex in given [`Aabb`] individually by this transformation matrix,
    /// under the assumption that the transformation basis is orthonormal (i.e. rotation/reflection is fine, scaling/skew is not),
    /// and then creates an `Aabb` encompassing all of them.
    ///
    /// For transforming by inverse of an affine transformation (e.g. with scaling) `transform.affine_inverse() * aabb` can be used instead. See [`Transform3D::affine_inverse()`].
    ///
    /// _Godot equivalent: `aabb * transform`_
    fn xform_inv(&self, rhs: Aabb) -> Aabb {
        // Same as Godot's `Transform3D::xform_inv` but omits unnecessary `Aabb::expand_to`.
        // There is probably some more clever way to do that.

        // Use the first vertex initialize our min/max.
        let end = self.xform_inv(rhs.end());
        // `min` is the "lowest" vertex of our Aabb, `max` is the farthest vertex.
        let (mut min, mut max) = (end, end);

        let vertices = [
            Vector3::new(
                rhs.position.x + rhs.size.x,
                rhs.position.y + rhs.size.y,
                rhs.position.z,
            ),
            Vector3::new(
                rhs.position.x + rhs.size.x,
                rhs.position.y,
                rhs.position.z + rhs.size.z,
            ),
            Vector3::new(rhs.position.x + rhs.size.x, rhs.position.y, rhs.position.z),
            Vector3::new(
                rhs.position.x,
                rhs.position.y + rhs.size.y,
                rhs.position.z + rhs.size.z,
            ),
            Vector3::new(rhs.position.x, rhs.position.y + rhs.size.y, rhs.position.z),
            Vector3::new(rhs.position.x, rhs.position.y, rhs.position.z + rhs.size.z),
            rhs.position,
        ];

        for v in vertices {
            let transformed = self.xform_inv(v);
            min = Vector3::coord_min(min, transformed);
            max = Vector3::coord_max(max, transformed);
        }

        Aabb::new(min, max - min)
    }
}

impl Mul<Plane> for Transform3D {
    type Output = Plane;

    fn mul(self, rhs: Plane) -> Self::Output {
        let point = self * (rhs.normal * rhs.d);

        let basis = self.basis.inverse().transposed();

        Plane::from_point_normal(point, (basis * rhs.normal).normalized())
    }
}

impl XformInv<Plane> for Transform3D {
    /// Inversely transforms (multiplies) the Plane by the given Transform3D transformation matrix.
    ///
    /// `transform.xform_inv(plane)` is equivalent to `transform.affine_inverse() * plane`. See [`Transform3D::affine_inverse()`].
    ///
    /// _Godot equivalent: `plane * transform`_
    fn xform_inv(&self, rhs: Plane) -> Plane {
        self.affine_inverse() * rhs
    }
}

impl ApproxEq for Transform3D {
    /// Returns if the two transforms are approximately equal, by comparing `basis` and `origin` separately.
    fn approx_eq(&self, other: &Self) -> bool {
        Basis::approx_eq(&self.basis, &other.basis)
            && Vector3::approx_eq(&self.origin, &other.origin)
    }
}

impl GlamType for RAffine3 {
    type Mapped = Transform3D;

    fn to_front(&self) -> Self::Mapped {
        Transform3D::new(self.matrix3.to_front(), self.translation.to_front())
    }

    // When `double-precision` is enabled this will complain. But it is
    // needed for when it is not enabled.
    #[allow(clippy::useless_conversion)]
    fn from_front(mapped: &Self::Mapped) -> Self {
        Self {
            matrix3: mapped.basis.to_glam().into(),
            translation: mapped.origin.to_glam().into(),
        }
    }
}

impl GlamConv for Transform3D {
    type Glam = RAffine3;
}

// SAFETY:
// This type is represented as `Self` in Godot, so `*mut Self` is sound.
unsafe impl GodotFfi for Transform3D {
    const VARIANT_TYPE: ExtVariantType = ExtVariantType::Concrete(sys::VariantType::TRANSFORM3D);

    ffi_methods! { type sys::GDExtensionTypePtr = *mut Self; .. }
}

crate::meta::impl_godot_as_self!(Transform3D: ByValue);

#[cfg(test)]
mod test {
    use super::*;

    // Tests translated from Godot.

    const DUMMY_TRANSFORM: Transform3D = Transform3D::new(
        Basis::from_cols(
            Vector3::new(1.0, 2.0, 3.0),
            Vector3::new(4.0, 5.0, 6.0),
            Vector3::new(7.0, 8.0, 9.0),
        ),
        Vector3::new(10.0, 11.0, 12.0),
    );

    #[test]
    fn translation() {
        let offset = Vector3::new(1.0, 2.0, 3.0);

        // Both versions should give the same result applied to identity.
        assert_eq!(
            Transform3D::IDENTITY.translated(offset),
            Transform3D::IDENTITY.translated_local(offset)
        );

        // Check both versions against left and right multiplications.
        let t = Transform3D::IDENTITY.translated(offset);
        assert_eq!(DUMMY_TRANSFORM.translated(offset), t * DUMMY_TRANSFORM);
        assert_eq!(
            DUMMY_TRANSFORM.translated_local(offset),
            DUMMY_TRANSFORM * t
        );
    }

    #[test]
    fn scaling() {
        let scaling = Vector3::new(1.0, 2.0, 3.0);

        // Both versions should give the same result applied to identity.
        assert_eq!(
            Transform3D::IDENTITY.scaled(scaling),
            Transform3D::IDENTITY.scaled_local(scaling)
        );

        // Check both versions against left and right multiplications.
        let s = Transform3D::IDENTITY.scaled(scaling);
        assert_eq!(DUMMY_TRANSFORM.scaled(scaling), s * DUMMY_TRANSFORM);
        assert_eq!(DUMMY_TRANSFORM.scaled_local(scaling), DUMMY_TRANSFORM * s);
    }

    #[test]
    fn rotation() {
        let axis = Vector3::new(1.0, 2.0, 3.0).normalized();
        let phi: real = 1.0;

        // Both versions should give the same result applied to identity.
        assert_eq!(
            Transform3D::IDENTITY.rotated(axis, phi),
            Transform3D::IDENTITY.rotated_local(axis, phi)
        );

        // Check both versions against left and right multiplications.
        let r = Transform3D::IDENTITY.rotated(axis, phi);
        assert_eq!(DUMMY_TRANSFORM.rotated(axis, phi), r * DUMMY_TRANSFORM);
        assert_eq!(
            DUMMY_TRANSFORM.rotated_local(axis, phi),
            DUMMY_TRANSFORM * r
        );
    }

    #[test]
    fn finite_number_checks() {
        let y = Vector3::new(0.0, 1.0, 2.0);
        let infinite_vec = Vector3::new(real::NAN, real::NAN, real::NAN);
        let x = Basis::from_rows(y, y, y);
        let infinite_basis = Basis::from_rows(infinite_vec, infinite_vec, infinite_vec);

        assert!(
            Transform3D::new(x, y).is_finite(),
            "Transform3D with all components finite should be finite",
        );

        assert!(
            !Transform3D::new(x, infinite_vec).is_finite(),
            "Transform3D with one component infinite should not be finite.",
        );
        assert!(
            !Transform3D::new(infinite_basis, y).is_finite(),
            "Transform3D with one component infinite should not be finite.",
        );

        assert!(
            !Transform3D::new(infinite_basis, infinite_vec).is_finite(),
            "Transform3D with two components infinite should not be finite.",
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_roundtrip() {
        let transform = Transform3D::default();
        let expected_json = "{\"basis\":{\"rows\":[{\"x\":1.0,\"y\":0.0,\"z\":0.0},{\"x\":0.0,\"y\":1.0,\"z\":0.0},{\"x\":0.0,\"y\":0.0,\"z\":1.0}]},\"origin\":{\"x\":0.0,\"y\":0.0,\"z\":0.0}}";

        crate::builtin::test_utils::roundtrip(&transform, expected_json);
    }
}
