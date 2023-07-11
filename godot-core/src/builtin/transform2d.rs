/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot_ffi as sys;
use sys::{ffi_methods, GodotFfi};

use crate::builtin::math::{assert_ne_approx, ApproxEq, FloatExt, GlamConv, GlamType};
use crate::builtin::real_consts::PI;
use crate::builtin::{real, RAffine2, RMat2, Rect2, Vector2};

use std::fmt::Display;
use std::ops::{Mul, MulAssign};

/// Affine 2D transform (2x3 matrix).
///
/// Represents transformations such as translation, rotation, or scaling.
///
/// Expressed as a 2x3 matrix, this transform consists of a two column vectors
/// `a` and `b` representing the basis of the transform, as well as the origin:
/// ```text
/// [ a.x  b.x  origin.x ]
/// [ a.y  b.y  origin.y ]
/// ```
///
/// For methods that don't take translation into account, see [`Basis2D`].
#[derive(Default, Copy, Clone, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(C)]
pub struct Transform2D {
    /// The first basis vector.
    ///
    /// This is equivalent to the `x` field from godot.
    pub a: Vector2,

    /// The second basis vector.
    ///
    /// This is equivalent to the `y` field from godot.
    pub b: Vector2,

    /// The origin of the transform. The coordinate space defined by this transform
    /// starts at this point.
    ///
    /// _Godot equivalent: `Transform2D.origin`_
    pub origin: Vector2,
}

impl Transform2D {
    /// The identity transform, with no translation, rotation or scaling
    /// applied. When applied to other data structures, `IDENTITY` performs no
    /// transformation.
    ///
    /// _Godot equivalent: `Transform2D.IDENTITY`_
    pub const IDENTITY: Self = Self::from_basis_origin(Basis2D::IDENTITY, Vector2::ZERO);

    /// The `Transform2D` that will flip something along its x axis.
    ///
    /// _Godot equivalent: `Transform2D.FLIP_X`_
    pub const FLIP_X: Self = Self::from_basis_origin(Basis2D::FLIP_X, Vector2::ZERO);

    /// The `Transform2D` that will flip something along its y axis.
    ///
    /// _Godot equivalent: `Transform2D.FLIP_Y`_
    pub const FLIP_Y: Self = Self::from_basis_origin(Basis2D::FLIP_Y, Vector2::ZERO);

    const fn from_basis_origin(basis: Basis2D, origin: Vector2) -> Self {
        let [a, b] = basis.cols;
        Self { a, b, origin }
    }

    /// Create a new `Transform2D` with the given column vectors.
    ///
    /// _Godot equivalent: `Transform2D(Vector2 x_axis, Vector2 y_axis, Vector2 origin)_
    pub const fn from_cols(a: Vector2, b: Vector2, origin: Vector2) -> Self {
        Self { a, b, origin }
    }

    /// Create a new `Transform2D` which will rotate by the given angle.
    pub fn from_angle(angle: real) -> Self {
        Self::from_angle_origin(angle, Vector2::ZERO)
    }

    /// Create a new `Transform2D` which will rotate by `angle` and translate
    /// by `origin`.
    ///
    /// _Godot equivalent: `Transform2D(float rotation, Vector2 position)`_
    pub fn from_angle_origin(angle: real, origin: Vector2) -> Self {
        Self::from_basis_origin(Basis2D::from_angle(angle), origin)
    }

    /// Create a new `Transform2D` which will rotate by `angle`, scale by
    /// `scale`, skew by `skew` and translate by `origin`.
    ///
    /// _Godot equivalent: `Transform2D(float rotation, Vector2 scale, float skew, Vector2 position)`_
    pub fn from_angle_scale_skew_origin(
        angle: real,
        scale: Vector2,
        skew: real,
        origin: Vector2,
    ) -> Self {
        // Translated from Godot's implementation

        Self::from_basis_origin(
            Basis2D::from_cols(
                Vector2::new(angle.cos(), angle.sin()),
                Vector2::new(-(angle + skew).sin(), (angle + skew).cos()),
            )
            .scaled(scale),
            origin,
        )
    }

    /// Unstable, used to simplify codegen. Too many parameters for public API and easy to have off-by-one, `from_cols()` is preferred.
    #[doc(hidden)]
    #[rustfmt::skip]
    #[allow(clippy::too_many_arguments)]
    pub const fn __internal_codegen(
       ax: real, ay: real,
       bx: real, by: real,
       ox: real, oy: real,
    ) -> Self {
        Self::from_cols(
            Vector2::new(ax, ay),
            Vector2::new(bx, by),
            Vector2::new(ox, oy),
        )
    }
    /// Create a reference to the first two columns of the transform
    /// interpreted as a [`Basis2D`].
    fn basis<'a>(&'a self) -> &'a Basis2D {
        // SAFETY: Both `Basis2D` and `Transform2D` are `repr(C)`, and the
        // layout of `Basis2D` is a prefix of `Transform2D`

        unsafe { std::mem::transmute::<&'a Transform2D, &'a Basis2D>(self) }
    }

    /// Create a [`Basis2D`] from the first two columns of the transform.
    fn to_basis(self) -> Basis2D {
        Basis2D::from_cols(self.a, self.b)
    }

    /// Returns the inverse of the transform, under the assumption that the
    /// transformation is composed of rotation, scaling and translation.
    ///
    /// _Godot equivalent: `Transform2D.affine_inverse()`_
    #[must_use]
    pub fn affine_inverse(self) -> Self {
        self.glam(|aff| aff.inverse())
    }

    /// Returns the transform's rotation (in radians).
    ///
    /// _Godot equivalent: `Transform2D.get_rotation()`_
    pub fn rotation(&self) -> real {
        self.basis().rotation()
    }

    /// Returns the transform's scale.
    ///
    /// _Godot equivalent: `Transform2D.get_scale()`_
    #[must_use]
    pub fn scale(&self) -> Vector2 {
        self.basis().scale()
    }

    /// Returns the transform's skew (in radians).
    ///
    /// _Godot equivalent: `Transform2D.get_skew()`_
    #[must_use]
    pub fn skew(&self) -> real {
        self.basis().skew()
    }

    /// Returns a transform interpolated between this transform and another by
    /// a given `weight` (on the range of 0.0 to 1.0).
    ///
    /// _Godot equivalent: `Transform2D.interpolate_with()`_
    #[must_use]
    pub fn interpolate_with(self, other: Self, weight: real) -> Self {
        Self::from_angle_scale_skew_origin(
            self.rotation().lerp_angle(other.rotation(), weight),
            self.scale().lerp(other.scale(), weight),
            self.skew().lerp_angle(other.skew(), weight),
            self.origin.lerp(other.origin, weight),
        )
    }

    /// Returns `true` if this transform is finite, by calling
    /// [`Vector2::is_finite()`] on each component.
    ///
    /// _Godot equivalent: `Transform2D.is_finite()`_
    pub fn is_finite(&self) -> bool {
        self.a.is_finite() && self.b.is_finite() && self.origin.is_finite()
    }

    /// Returns the transform with the basis orthogonal (90 degrees), and
    /// normalized axis vectors (scale of 1 or -1).
    ///
    /// _Godot equivalent: `Transform2D.orthonormalized()`_
    #[must_use]
    pub fn orthonormalized(self) -> Self {
        Self::from_basis_origin(self.basis().orthonormalized(), self.origin)
    }

    /// Returns a copy of the transform rotated by the given `angle` (in radians).
    /// This method is an optimized version of multiplying the given transform `X`
    /// with a corresponding rotation transform `R` from the left, i.e., `R * X`.
    /// This can be seen as transforming with respect to the global/parent frame.
    ///
    /// _Godot equivalent: `Transform2D.rotated()`_
    #[must_use]
    pub fn rotated(self, angle: real) -> Self {
        Self::from_angle(angle) * self
    }

    /// Returns a copy of the transform rotated by the given `angle` (in radians).
    /// This method is an optimized version of multiplying the given transform `X`
    /// with a corresponding rotation transform `R` from the right, i.e., `X * R`.
    /// This can be seen as transforming with respect to the local frame.
    ///
    /// _Godot equivalent: `Transform2D.rotated_local()`_
    #[must_use]
    pub fn rotated_local(self, angle: real) -> Self {
        self * Self::from_angle(angle)
    }

    /// Returns a copy of the transform scaled by the given scale factor.
    /// This method is an optimized version of multiplying the given transform `X`
    /// with a corresponding scaling transform `S` from the left, i.e., `S * X`.
    /// This can be seen as transforming with respect to the global/parent frame.
    ///
    /// _Godot equivalent: `Transform2D.scaled()`_
    #[must_use]
    pub fn scaled(self, scale: Vector2) -> Self {
        let mut basis = self.to_basis();
        basis.set_row_a(basis.row_a() * scale.x);
        basis.set_row_b(basis.row_b() * scale.y);
        Self::from_basis_origin(basis, self.origin * scale)
    }

    /// Returns a copy of the transform scaled by the given scale factor.
    /// This method is an optimized version of multiplying the given transform `X`
    /// with a corresponding scaling transform `S` from the right, i.e., `X * S`.
    /// This can be seen as transforming with respect to the local frame.
    ///
    /// _Godot equivalent: `Transform2D.scaled_local()`_
    #[must_use]
    pub fn scaled_local(self, scale: Vector2) -> Self {
        Self::from_basis_origin(self.basis().scaled(scale), self.origin)
    }

    /// Returns a copy of the transform translated by the given offset.
    /// This method is an optimized version of multiplying the given transform `X`
    /// with a corresponding translation transform `T` from the left, i.e., `T * X`.
    /// This can be seen as transforming with respect to the global/parent frame.
    ///
    /// _Godot equivalent: `Transform2D.translated()`_
    #[must_use]
    pub fn translated(self, offset: Vector2) -> Self {
        Self::from_cols(self.a, self.b, self.origin + offset)
    }

    /// Returns a copy of the transform translated by the given offset.
    /// This method is an optimized version of multiplying the given transform `X`
    /// with a corresponding translation transform `T` from the right, i.e., `X * T`.
    /// This can be seen as transforming with respect to the local frame.
    ///
    /// _Godot equivalent: `Transform2D.translated()`_
    #[must_use]
    pub fn translated_local(self, offset: Vector2) -> Self {
        Self::from_cols(self.a, self.b, self.origin + (self.to_basis() * offset))
    }

    /// Returns a vector transformed (multiplied) by the basis matrix.
    /// This method does not account for translation (the origin vector).
    ///
    /// _Godot equivalent: `Transform2D.basis_xform()`_
    pub fn basis_xform(&self, v: Vector2) -> Vector2 {
        self.to_basis() * v
    }

    /// Returns a vector transformed (multiplied) by the inverse basis matrix.
    /// This method does not account for translation (the origin vector).
    ///
    /// _Godot equivalent: `Transform2D.basis_xform_inv()`_
    pub fn basis_xform_inv(&self, v: Vector2) -> Vector2 {
        self.basis().inverse() * v
    }
}

impl Display for Transform2D {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Godot output:
        // [X: (1, 2), Y: (3, 4), O: (5, 6)]
        // Where X,Y,O are the columns

        let Transform2D { a, b, origin } = self;

        write!(f, "[a: {a}, b: {b}, o: {origin}]")
    }
}

impl Mul for Transform2D {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        self.glam2(&rhs, |a, b| a * b)
    }
}

impl Mul<Vector2> for Transform2D {
    type Output = Vector2;

    fn mul(self, rhs: Vector2) -> Self::Output {
        self.glam2(&rhs, |t, v| t.transform_point2(v))
    }
}

impl Mul<real> for Transform2D {
    type Output = Self;

    fn mul(self, rhs: real) -> Self::Output {
        Self::from_cols(self.a * rhs, self.b * rhs, self.origin * rhs)
    }
}

impl Mul<Rect2> for Transform2D {
    type Output = Rect2;

    /// Transforms each coordinate in `rhs.position` and `rhs.end()` individually by this transform, then
    /// creates a `Rect2` containing all of them.
    fn mul(self, rhs: Rect2) -> Self::Output {
        // https://web.archive.org/web/20220317024830/https://dev.theomader.com/transform-bounding-boxes/
        let xa = self.a * rhs.position.x;
        let xb = self.a * rhs.end().x;

        let ya = self.b * rhs.position.y;
        let yb = self.b * rhs.end().y;

        let position = Vector2::coord_min(xa, xb) + Vector2::coord_min(ya, yb) + self.origin;
        let end = Vector2::coord_max(xa, xb) + Vector2::coord_max(ya, yb) + self.origin;
        Rect2::new(position, end - position)
    }
}

impl ApproxEq for Transform2D {
    /// Returns if the two transforms are approximately equal, by comparing each component separately.
    #[inline]
    fn approx_eq(&self, other: &Self) -> bool {
        Vector2::approx_eq(&self.a, &other.a)
            && Vector2::approx_eq(&self.b, &other.b)
            && Vector2::approx_eq(&self.origin, &other.origin)
    }
}

impl GlamType for RAffine2 {
    type Mapped = Transform2D;

    fn to_front(&self) -> Self::Mapped {
        Transform2D::from_basis_origin(self.matrix2.to_front(), self.translation.to_front())
    }

    fn from_front(mapped: &Self::Mapped) -> Self {
        Self {
            matrix2: mapped.basis().to_glam(),
            translation: mapped.origin.to_glam(),
        }
    }
}

impl GlamConv for Transform2D {
    type Glam = RAffine2;
}

// SAFETY:
// This type is represented as `Self` in Godot, so `*mut Self` is sound.
unsafe impl GodotFfi for Transform2D {
    ffi_methods! { type sys::GDExtensionTypePtr = *mut Self; .. }
}

/// A 2x2 matrix, typically used as an orthogonal basis for [`Transform2D`].
///
/// Indexing into a `Basis2D` is done in a column-major order, meaning that
/// `basis[0]` is the first basis-vector.
///
/// This has no direct equivalent in Godot, but is the same as the `x` and `y`
/// vectors from a `Transform2D`.
#[derive(Copy, Clone, PartialEq, Debug)]
#[repr(C)]
pub(crate) struct Basis2D {
    /// The columns of the matrix.
    cols: [Vector2; 2],
}

impl Basis2D {
    /// The identity basis, with no rotation or scaling applied.
    pub(crate) const IDENTITY: Self = Self::from_diagonal(1.0, 1.0);

    /// The basis that will flip something along the X axis when used in a
    /// transformation.
    pub(crate) const FLIP_X: Self = Self::from_diagonal(-1.0, 1.0);

    /// The basis that will flip something along the X axis when used in a
    /// transformation.
    pub(crate) const FLIP_Y: Self = Self::from_diagonal(1.0, -1.0);

    /// Create a diagonal matrix from the given values.
    pub(crate) const fn from_diagonal(x: real, y: real) -> Self {
        Self::from_cols(Vector2::new(x, 0.0), Vector2::new(0.0, y))
    }

    /// Create a new basis from 2 basis vectors.
    pub(crate) const fn from_cols(x: Vector2, y: Vector2) -> Self {
        Self { cols: [x, y] }
    }

    /// Create a `Basis2D` from an angle.
    pub(crate) fn from_angle(angle: real) -> Self {
        RMat2::from_angle(angle).to_front()
    }

    /// Returns the scale of the matrix.
    #[must_use]
    pub(crate) fn scale(&self) -> Vector2 {
        let det_sign = self.determinant().signum();
        Vector2::new(self.cols[0].length(), det_sign * self.cols[1].length())
    }

    /// Introduces an additional scaling.
    #[must_use]
    pub(crate) fn scaled(self, scale: Vector2) -> Self {
        Self {
            cols: [self.cols[0] * scale.x, self.cols[1] * scale.y],
        }
    }

    /// Returns the determinant of the matrix.
    pub(crate) fn determinant(&self) -> real {
        self.glam(|mat| mat.determinant())
    }

    /// Returns the inverse of the matrix.
    #[must_use]
    pub fn inverse(self) -> Self {
        self.glam(|mat| mat.inverse())
    }

    /// Returns the orthonormalized version of the basis.
    #[must_use]
    pub(crate) fn orthonormalized(self) -> Self {
        assert_ne_approx!(self.determinant(), 0.0, "Determinant should not be zero.");

        // Gram-Schmidt Process
        let mut x = self.cols[0];
        let mut y = self.cols[1];

        x = x.normalized();
        y = y - x * (x.dot(y));
        y = y.normalized();

        Self::from_cols(x, y)
    }

    /// Returns the rotation of the matrix
    pub(crate) fn rotation(&self) -> real {
        // Translated from Godot
        real::atan2(self.cols[0].y, self.cols[0].x)
    }

    /// Returns the skew of the matrix
    #[must_use]
    pub(crate) fn skew(&self) -> real {
        // Translated from Godot
        let det_sign = self.determinant().signum();
        self.cols[0]
            .normalized()
            .dot(det_sign * self.cols[1].normalized())
            .acos()
            - PI * 0.5
    }

    pub(crate) fn set_row_a(&mut self, v: Vector2) {
        self.cols[0].x = v.x;
        self.cols[1].x = v.y;
    }

    pub(crate) fn row_a(&self) -> Vector2 {
        Vector2::new(self.cols[0].x, self.cols[1].x)
    }

    pub(crate) fn set_row_b(&mut self, v: Vector2) {
        self.cols[0].y = v.x;
        self.cols[1].y = v.y;
    }

    pub(crate) fn row_b(&self) -> Vector2 {
        Vector2::new(self.cols[0].y, self.cols[1].y)
    }
}

impl Default for Basis2D {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Display for Basis2D {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let [a, b] = self.cols;
        write!(f, "[a: {a}, b: {b})]")
    }
}

impl Mul for Basis2D {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        self.glam2(&rhs, |a, b| a * b)
    }
}

impl Mul<real> for Basis2D {
    type Output = Self;

    fn mul(self, rhs: real) -> Self::Output {
        (self.to_glam() * rhs).to_front()
    }
}

impl MulAssign<real> for Basis2D {
    fn mul_assign(&mut self, rhs: real) {
        self.cols[0] *= rhs;
        self.cols[1] *= rhs;
    }
}

impl Mul<Vector2> for Basis2D {
    type Output = Vector2;

    fn mul(self, rhs: Vector2) -> Self::Output {
        self.glam2(&rhs, |a, b| a * b)
    }
}

impl GlamType for RMat2 {
    type Mapped = Basis2D;

    fn to_front(&self) -> Self::Mapped {
        Basis2D {
            cols: [self.col(0).to_front(), self.col(1).to_front()],
        }
    }

    fn from_front(mapped: &Self::Mapped) -> Self {
        Self::from_cols(mapped.cols[0].to_glam(), mapped.cols[1].to_glam())
    }
}

impl GlamConv for Basis2D {
    type Glam = RMat2;
}

#[cfg(test)]
mod test {
    use crate::assert_eq_approx;

    use super::*;

    #[test]
    fn transform2d_constructors_correct() {
        let trans = Transform2D::from_angle(real!(115.0).to_radians());
        assert_eq_approx!(trans.rotation(), real!(115.0).to_radians());

        let trans =
            Transform2D::from_angle_origin(real!(-80.0).to_radians(), Vector2::new(1.4, 9.8));
        assert_eq_approx!(trans.rotation(), real!(-80.0).to_radians());
        assert_eq_approx!(trans.origin, Vector2::new(1.4, 9.8));

        let trans = Transform2D::from_angle_scale_skew_origin(
            real!(170.0).to_radians(),
            Vector2::new(3.6, 8.0),
            real!(20.0).to_radians(),
            Vector2::new(2.4, 6.8),
        );
        assert_eq_approx!(trans.rotation(), real!(170.0).to_radians());
        assert_eq_approx!(trans.scale(), Vector2::new(3.6, 8.0));
        assert_eq_approx!(trans.skew(), real!(20.0).to_radians());
        assert_eq_approx!(trans.origin, Vector2::new(2.4, 6.8));
    }

    // Tests translated from Godot.

    const DUMMY_TRANSFORM: Transform2D = Transform2D::from_basis_origin(
        Basis2D::from_cols(Vector2::new(1.0, 2.0), Vector2::new(3.0, 4.0)),
        Vector2::new(5.0, 6.0),
    );

    #[test]
    fn translation() {
        let offset = Vector2::new(1.0, 2.0);

        // Both versions should give the same result applied to identity.
        assert_eq!(
            Transform2D::IDENTITY.translated(offset),
            Transform2D::IDENTITY.translated_local(offset)
        );

        // Check both versions against left and right multiplications.
        let t = Transform2D::IDENTITY.translated(offset);
        assert_eq!(DUMMY_TRANSFORM.translated(offset), t * DUMMY_TRANSFORM);
        assert_eq!(
            DUMMY_TRANSFORM.translated_local(offset),
            DUMMY_TRANSFORM * t
        );
    }

    #[test]
    fn scaling() {
        let scaling = Vector2::new(1.0, 2.0);

        // Both versions should give the same result applied to identity.
        assert_eq!(
            Transform2D::IDENTITY.scaled(scaling),
            Transform2D::IDENTITY.scaled_local(scaling)
        );

        // Check both versions against left and right multiplications.
        let s: Transform2D = Transform2D::IDENTITY.scaled(scaling);
        assert_eq!(DUMMY_TRANSFORM.scaled(scaling), s * DUMMY_TRANSFORM);
        assert_eq!(DUMMY_TRANSFORM.scaled_local(scaling), DUMMY_TRANSFORM * s);
    }

    #[test]
    fn rotation() {
        let phi = 1.0;

        // Both versions should give the same result applied to identity.
        assert_eq!(
            Transform2D::IDENTITY.rotated(phi),
            Transform2D::IDENTITY.rotated_local(phi)
        );

        // Check both versions against left and right multiplications.
        let r: Transform2D = Transform2D::IDENTITY.rotated(phi);
        assert_eq!(DUMMY_TRANSFORM.rotated(phi), r * DUMMY_TRANSFORM);
        assert_eq!(DUMMY_TRANSFORM.rotated_local(phi), DUMMY_TRANSFORM * r);
    }

    #[test]
    fn interpolation() {
        let rotate_scale_skew_pos: Transform2D = Transform2D::from_angle_scale_skew_origin(
            real!(170.0).to_radians(),
            Vector2::new(3.6, 8.0),
            real!(20.0).to_radians(),
            Vector2::new(2.4, 6.8),
        );

        let rotate_scale_skew_pos_halfway: Transform2D = Transform2D::from_angle_scale_skew_origin(
            real!(85.0).to_radians(),
            Vector2::new(2.3, 4.5),
            real!(10.0).to_radians(),
            Vector2::new(1.2, 3.4),
        );

        let interpolated: Transform2D =
            Transform2D::IDENTITY.interpolate_with(rotate_scale_skew_pos, 0.5);
        assert_eq_approx!(interpolated.origin, rotate_scale_skew_pos_halfway.origin);
        assert_eq_approx!(
            interpolated.rotation(),
            rotate_scale_skew_pos_halfway.rotation(),
        );
        assert_eq_approx!(interpolated.scale(), rotate_scale_skew_pos_halfway.scale());
        assert_eq_approx!(interpolated.skew(), rotate_scale_skew_pos_halfway.skew());
        assert_eq_approx!(interpolated, rotate_scale_skew_pos_halfway);

        let interpolated = rotate_scale_skew_pos.interpolate_with(Transform2D::IDENTITY, 0.5);
        assert_eq_approx!(interpolated, rotate_scale_skew_pos_halfway);
    }

    #[test]
    fn finite_number_checks() {
        let x: Vector2 = Vector2::new(0.0, 1.0);
        let infinite: Vector2 = Vector2::new(real::NAN, real::NAN);

        assert!(
            Transform2D::from_basis_origin(Basis2D::from_cols(x, x), x).is_finite(),
            "let with: Transform2D all components finite should be finite",
        );

        assert!(
            !Transform2D::from_basis_origin(Basis2D::from_cols(infinite, x), x).is_finite(),
            "let with: Transform2D one component infinite should not be finite.",
        );
        assert!(
            !Transform2D::from_basis_origin(Basis2D::from_cols(x, infinite), x).is_finite(),
            "let with: Transform2D one component infinite should not be finite.",
        );
        assert!(
            !Transform2D::from_basis_origin(Basis2D::from_cols(x, x), infinite).is_finite(),
            "let with: Transform2D one component infinite should not be finite.",
        );

        assert!(
            !Transform2D::from_basis_origin(Basis2D::from_cols(infinite, infinite), x).is_finite(),
            "let with: Transform2D two components infinite should not be finite.",
        );
        assert!(
            !Transform2D::from_basis_origin(Basis2D::from_cols(infinite, x), infinite).is_finite(),
            "let with: Transform2D two components infinite should not be finite.",
        );
        assert!(
            !Transform2D::from_basis_origin(Basis2D::from_cols(x, infinite), infinite).is_finite(),
            "let with: Transform2D two components infinite should not be finite.",
        );

        assert!(
            !Transform2D::from_basis_origin(Basis2D::from_cols(infinite, infinite), infinite)
                .is_finite(),
            "let with: Transform2D three components infinite should not be finite.",
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_roundtrip() {
        let transform = Transform2D::default();
        let expected_json = "{\"a\":{\"x\":0.0,\"y\":0.0},\"b\":{\"x\":0.0,\"y\":0.0},\"origin\":{\"x\":0.0,\"y\":0.0}}";

        crate::builtin::test_utils::roundtrip(&transform, expected_json);
    }
}
