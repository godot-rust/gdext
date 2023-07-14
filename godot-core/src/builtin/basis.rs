/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot_ffi as sys;
use sys::{ffi_methods, GodotFfi};

use crate::builtin::math::{ApproxEq, FloatExt, GlamConv, GlamType};
use crate::builtin::real_consts::FRAC_PI_2;
use crate::builtin::{real, Quaternion, RMat3, RQuat, RVec2, RVec3, Vector3};

use std::cmp::Ordering;
use std::fmt::Display;
use std::ops::{Mul, MulAssign};

/// A 3x3 matrix, typically used as an orthogonal basis for [`Transform3D`](crate::builtin::Transform3D).
///
/// Indexing into a `Basis` is done in row-major order. So `mat[1]` would return the first *row* and not
/// the first *column*/basis vector. This means that indexing into the matrix happens in the same order
/// it usually does in math, except that we index starting at 0.
///
/// The basis vectors are the columns of the matrix, whereas the [`rows`](Self::rows) field represents
/// the row vectors.
#[derive(Copy, Clone, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(C)]
pub struct Basis {
    /// The rows of the matrix. These are *not* the basis vectors.  
    ///
    /// To access the basis vectors see [`col_a()`](Self::col_a), [`set_col_a()`](Self::set_col_a),
    /// [`col_b()`](Self::col_b), [`set_col_b()`](Self::set_col_b), [`col_c`](Self::col_c()),
    /// [`set_col_c()`](Self::set_col_c).
    pub rows: [Vector3; 3],
}

impl Basis {
    /// The identity basis, with no rotation or scaling applied.
    ///
    /// _Godot equivalent: `Basis.IDENTITY`_
    pub const IDENTITY: Self = Self::from_diagonal(1.0, 1.0, 1.0);

    /// The basis that will flip something along the X axis when used in a transformation.
    ///
    /// _Godot equivalent: `Basis.FLIP_X`_
    pub const FLIP_X: Self = Self::from_diagonal(-1.0, 1.0, 1.0);

    /// The basis that will flip something along the Y axis when used in a transformation.
    ///
    /// _Godot equivalent: `Basis.FLIP_Y`_
    pub const FLIP_Y: Self = Self::from_diagonal(1.0, -1.0, 1.0);

    /// The basis that will flip something along the Z axis when used in a transformation.
    ///
    /// _Godot equivalent: `Basis.FLIP_Z`_
    pub const FLIP_Z: Self = Self::from_diagonal(1.0, 1.0, -1.0);

    /// Create a new basis from 3 row vectors. These are *not* basis vectors.
    pub const fn from_rows(x: Vector3, y: Vector3, z: Vector3) -> Self {
        Self { rows: [x, y, z] }
    }

    /// Create a new basis from 3 column vectors.
    pub const fn from_cols(a: Vector3, b: Vector3, c: Vector3) -> Self {
        Self::from_rows_array(&[a.x, b.x, c.x, a.y, b.y, c.y, a.z, b.z, c.z])
    }

    /// Create a `Basis` from an axis and angle.
    ///
    /// _Godot equivalent: `Basis(Vector3 axis, float angle)`_
    pub fn from_axis_angle(axis: Vector3, angle: real) -> Self {
        RMat3::from_axis_angle(axis.to_glam(), angle).to_front()
    }

    /// Create a diagonal matrix from the given values.
    pub const fn from_diagonal(x: real, y: real, z: real) -> Self {
        Self {
            rows: [
                Vector3::new(x, 0.0, 0.0),
                Vector3::new(0.0, y, 0.0),
                Vector3::new(0.0, 0.0, z),
            ],
        }
    }

    /// Create a diagonal matrix from the given values.
    ///
    /// _Godot equivalent: `Basis.from_scale(Vector3 scale)`
    pub const fn from_scale(scale: Vector3) -> Self {
        Self::from_diagonal(scale.x, scale.y, scale.z)
    }

    const fn from_rows_array(rows: &[real; 9]) -> Self {
        let [ax, bx, cx, ay, by, cy, az, bz, cz] = rows;
        Self::from_rows(
            Vector3::new(*ax, *bx, *cx),
            Vector3::new(*ay, *by, *cy),
            Vector3::new(*az, *bz, *cz),
        )
    }

    /// Create a `Basis` from a `Quaternion`.
    ///
    /// _Godot equivalent: `Basis(Quaternion from)`_
    pub fn from_quat(quat: Quaternion) -> Self {
        RMat3::from_quat(quat.to_glam()).to_front()
    }

    /// Create a `Basis` from three angles `a`, `b`, and `c` interpreted
    /// as Euler angles according to the given `EulerOrder`.
    ///
    /// _Godot equivalent: `Basis.from_euler(Vector3 euler, int order)`_
    pub fn from_euler(order: EulerOrder, angles: Vector3) -> Self {
        // Translated from "Basis::from_euler" in
        // https://github.com/godotengine/godot/blob/master/core/math/basis.cpp

        // We can't use glam to do these conversions since glam uses intrinsic rotations
        // whereas godot uses extrinsic rotations.
        // see https://github.com/bitshifter/glam-rs/issues/337
        let Vector3 { x: a, y: b, z: c } = angles;
        let xmat =
            Basis::from_rows_array(&[1.0, 0.0, 0.0, 0.0, a.cos(), -a.sin(), 0.0, a.sin(), a.cos()]);
        let ymat =
            Basis::from_rows_array(&[b.cos(), 0.0, b.sin(), 0.0, 1.0, 0.0, -b.sin(), 0.0, b.cos()]);
        let zmat =
            Basis::from_rows_array(&[c.cos(), -c.sin(), 0.0, c.sin(), c.cos(), 0.0, 0.0, 0.0, 1.0]);

        match order {
            EulerOrder::XYZ => xmat * ymat * zmat,
            EulerOrder::XZY => xmat * zmat * ymat,
            EulerOrder::YXZ => ymat * xmat * zmat,
            EulerOrder::YZX => ymat * zmat * xmat,
            EulerOrder::ZXY => zmat * xmat * ymat,
            EulerOrder::ZYX => zmat * ymat * xmat,
        }
    }

    /// Creates a `Basis` with a rotation such that the forward axis (-Z) points
    /// towards the `target` position.
    ///
    /// The up axis (+Y) points as close to the `up` vector as possible while
    /// staying perpendicular to the forward axis. The resulting Basis is
    /// orthonormalized. The `target` and `up` vectors cannot be zero, and
    /// cannot be parallel to each other.
    ///
    #[cfg(before_api = "4.1")]
    /// _Godot equivalent: `Basis.looking_at()`_
    #[doc(alias = "looking_at")]
    pub fn new_looking_at(target: Vector3, up: Vector3) -> Self {
        super::inner::InnerBasis::looking_at(target, up)
    }

    /// If `use_model_front` is true, the +Z axis (asset front) is treated as forward (implies +X is left)
    /// and points toward the target position. By default, the -Z axis (camera forward) is treated as forward
    /// (implies +X is right).
    ///
    /// _Godot equivalent: `Basis.looking_at()`_
    #[cfg(since_api = "4.1")]
    pub fn new_looking_at(target: Vector3, up: Vector3, use_model_front: bool) -> Self {
        super::inner::InnerBasis::looking_at(target, up, use_model_front)
    }

    /// Creates a `[Vector3; 3]` with the columns of the `Basis`.
    pub fn to_cols(self) -> [Vector3; 3] {
        self.transposed().rows
    }

    /// Creates a [`Quaternion`] representing the same rotation as this basis.
    ///
    /// _Godot equivalent: `Basis.get_rotation_quaternion()`_
    #[doc(alias = "get_rotation_quaternion")]
    pub fn to_quat(self) -> Quaternion {
        RQuat::from_mat3(&self.orthonormalized().to_glam()).to_front()
    }

    const fn to_rows_array(self) -> [real; 9] {
        let [Vector3 {
            x: ax,
            y: bx,
            z: cx,
        }, Vector3 {
            x: ay,
            y: by,
            z: cy,
        }, Vector3 {
            x: az,
            y: bz,
            z: cz,
        }] = self.rows;
        [ax, bx, cx, ay, by, cy, az, bz, cz]
    }

    /// Returns the scale of the matrix.
    ///
    /// _Godot equivalent: `Basis.get_scale()`_
    #[must_use]
    pub fn scale(&self) -> Vector3 {
        let det = self.determinant();
        let det_sign = if det < 0.0 { -1.0 } else { 1.0 };

        Vector3::new(
            self.col_a().length(),
            self.col_b().length(),
            self.col_c().length(),
        ) * det_sign
    }

    /// Returns the rotation of the matrix in euler angles.
    ///
    /// The order of the angles are given by `order`.
    ///
    /// _Godot equivalent: `Basis.get_euler()`_
    pub fn to_euler(self, order: EulerOrder) -> Vector3 {
        use glam::swizzles::Vec3Swizzles as _;

        let col_a = self.col_a().to_glam();
        let col_b = self.col_b().to_glam();
        let col_c = self.col_c().to_glam();

        let row_a = self.rows[0].to_glam();
        let row_b = self.rows[1].to_glam();
        let row_c = self.rows[2].to_glam();

        let major = match order {
            EulerOrder::XYZ => self.rows[0].z,
            EulerOrder::XZY => self.rows[0].y,
            EulerOrder::YXZ => self.rows[1].z,
            EulerOrder::YZX => self.rows[1].x,
            EulerOrder::ZXY => self.rows[2].y,
            EulerOrder::ZYX => self.rows[2].x,
        };

        // Return the simplest forms for pure rotations
        if let Some(pure_rotation) = match order {
            EulerOrder::XYZ => self
                .to_euler_pure_rotation(major, 1, row_a.zx())
                .map(RVec3::yxz),
            EulerOrder::YXZ => {
                self.to_euler_pure_rotation(major, 0, RVec2::new(-major, self.rows[1].y))
            }
            _ => None,
        } {
            return pure_rotation.to_front();
        }

        match order {
            EulerOrder::XYZ => {
                -Self::to_euler_inner(major, col_c.yz(), row_a.yx(), col_b.zy()).yxz()
            }
            EulerOrder::XZY => {
                Self::to_euler_inner(major, col_b.zy(), row_a.zx(), col_c.yz()).yzx()
            }
            EulerOrder::YXZ => {
                let mut vec = Self::to_euler_inner(major, col_c.xz(), row_b.xy(), row_a.yx());
                if Self::is_between_neg1_1(major).is_lt() {
                    vec.y = -vec.y;
                }
                vec
            }
            EulerOrder::YZX => {
                -Self::to_euler_inner(major, row_b.zy(), col_a.zx(), row_c.yz()).yzx()
            }
            EulerOrder::ZXY => {
                -Self::to_euler_inner(major, row_c.xz(), col_b.xy(), row_a.zx()).xyz()
            }
            EulerOrder::ZYX => {
                Self::to_euler_inner(major, col_a.yx(), row_c.yz(), col_b.xy()).zxy()
            }
        }
        .to_front()
    }

    fn is_between_neg1_1(f: real) -> Ordering {
        if f >= (1.0 - real::CMP_EPSILON) {
            Ordering::Greater
        } else if f <= -(1.0 - real::CMP_EPSILON) {
            Ordering::Less
        } else {
            Ordering::Equal
        }
    }

    /// Check if the element at `basis_{i,i}` is 1, and all the other values in
    /// that row and column are 0.
    fn is_identity_index(&self, index: usize) -> bool {
        let row = self.rows[index];
        let col = self.transposed().rows[index];
        if row != col {
            return false;
        }
        match index {
            0 => row == Vector3::RIGHT,
            1 => row == Vector3::UP,
            2 => row == Vector3::BACK,
            _ => panic!("Unknown Index {index}"),
        }
    }

    #[allow(clippy::wrong_self_convention)]
    fn to_euler_pure_rotation(
        &self,
        major: real,
        index: usize,
        rotation_vec: RVec2,
    ) -> Option<RVec3> {
        if Self::is_between_neg1_1(major).is_ne() {
            return None;
        }

        if !self.is_identity_index(index) {
            return None;
        }

        Some(RVec3::new(
            real::atan2(rotation_vec.x, rotation_vec.y),
            0.0,
            0.0,
        ))
    }

    fn to_euler_inner(major: real, pair0: RVec2, pair1: RVec2, pair2: RVec2) -> RVec3 {
        match Self::is_between_neg1_1(major) {
            // It's -1
            Ordering::Less => RVec3::new(FRAC_PI_2, -real::atan2(pair2.x, pair2.y), 0.0),
            // Is it a pure rotation?
            Ordering::Equal => RVec3::new(
                real::asin(-major),
                real::atan2(pair0.x, pair0.y),
                real::atan2(pair1.x, pair1.y),
            ),
            // It's 1
            Ordering::Greater => RVec3::new(-FRAC_PI_2, -real::atan2(pair2.x, pair2.y), 0.0),
        }
    }

    /// Returns the determinant of the matrix.
    ///
    /// _Godot equivalent: `Basis.determinant()`_
    pub fn determinant(&self) -> real {
        self.to_glam().determinant()
    }

    /// Introduce an additional scaling specified by the given 3D scaling factor.
    ///
    /// _Godot equivalent: `Basis.scaled()`_
    #[must_use]
    pub fn scaled(self, scale: Vector3) -> Self {
        Self::from_diagonal(scale.x, scale.y, scale.z) * self
    }

    /// Returns the inverse of the matrix.
    ///
    /// _Godot equivalent: `Basis.inverse()`_
    #[must_use]
    pub fn inverse(self) -> Basis {
        self.glam(|mat| mat.inverse())
    }

    /// Returns the transposed version of the matrix.
    ///
    /// _Godot equivalent: `Basis.transposed()`_
    #[must_use]
    pub fn transposed(self) -> Self {
        Self::from_cols(self.rows[0], self.rows[1], self.rows[2])
    }

    /// ⚠️ Returns the orthonormalized version of the matrix (useful to call from
    /// time to time to avoid rounding error for orthogonal matrices). This
    /// performs a Gram-Schmidt orthonormalization on the basis of the matrix.
    ///
    /// # Panics
    ///
    /// If the determinant of the matrix is 0.
    ///
    /// _Godot equivalent: `Basis.orthonormalized()`_
    #[must_use]
    pub fn orthonormalized(self) -> Self {
        assert!(
            !self.determinant().is_zero_approx(),
            "Determinant should not be zero."
        );

        // Gram-Schmidt Process
        let mut x = self.col_a();
        let mut y = self.col_b();
        let mut z = self.col_c();

        x = x.normalized();
        y = y - x * x.dot(y);
        y = y.normalized();
        z = z - x * x.dot(z) - y * y.dot(z);
        z = z.normalized();

        Self::from_cols(x, y, z)
    }

    /// Introduce an additional rotation around the given `axis` by `angle`
    /// (in radians). The axis must be a normalized vector.
    ///
    /// _Godot equivalent: `Basis.rotated()`_
    #[must_use]
    pub fn rotated(self, axis: Vector3, angle: real) -> Self {
        Self::from_axis_angle(axis, angle) * self
    }

    /// Assuming that the matrix is a proper rotation matrix, slerp performs
    /// a spherical-linear interpolation with another rotation matrix.
    ///
    /// _Godot equivalent: `Basis.slerp()`_
    #[must_use]
    pub fn slerp(self, other: Self, weight: real) -> Self {
        let from = self.to_quat();
        let to = other.to_quat();

        let mut result = Self::from_quat(from.slerp(to, weight));

        for i in 0..3 {
            result.rows[i] *= self.rows[i].length().lerp(other.rows[i].length(), weight);
        }

        result
    }

    /// Transposed dot product with the X axis (column) of the matrix.
    ///
    /// _Godot equivalent: `Basis.tdotx()`_
    #[must_use]
    pub fn tdotx(&self, with: Vector3) -> real {
        self.col_a().dot(with)
    }

    /// Transposed dot product with the Y axis (column) of the matrix.
    ///
    /// _Godot equivalent: `Basis.tdoty()`_
    #[must_use]
    pub fn tdoty(&self, with: Vector3) -> real {
        self.col_b().dot(with)
    }

    /// Transposed dot product with the Z axis (column) of the matrix.
    ///
    /// _Godot equivalent: `Basis.tdotz()`_
    #[must_use]
    pub fn tdotz(&self, with: Vector3) -> real {
        self.col_c().dot(with)
    }

    /// Returns `true` if this basis is finite. Meaning each element of the
    /// matrix is not `NaN`, positive infinity, or negative infinity.
    ///
    /// _Godot equivalent: `Basis.is_finite()`_
    pub fn is_finite(&self) -> bool {
        self.rows[0].is_finite() && self.rows[1].is_finite() && self.rows[2].is_finite()
    }

    /// Returns the first column of the matrix,
    ///
    /// _Godot equivalent: `Basis.x`_
    #[doc(alias = "x")]
    #[must_use]
    pub fn col_a(&self) -> Vector3 {
        Vector3::new(self.rows[0].x, self.rows[1].x, self.rows[2].x)
    }

    /// Set the values of the first column of the matrix.
    pub fn set_col_a(&mut self, col: Vector3) {
        self.rows[0].x = col.x;
        self.rows[1].x = col.y;
        self.rows[2].x = col.z;
    }

    /// Returns the second column of the matrix,
    ///
    /// _Godot equivalent: `Basis.y`_
    #[doc(alias = "y")]
    #[must_use]
    pub fn col_b(&self) -> Vector3 {
        Vector3::new(self.rows[0].y, self.rows[1].y, self.rows[2].y)
    }

    /// Set the values of the second column of the matrix.
    pub fn set_col_b(&mut self, col: Vector3) {
        self.rows[0].y = col.x;
        self.rows[1].y = col.y;
        self.rows[2].y = col.z;
    }

    /// Returns the third column of the matrix,
    ///
    /// _Godot equivalent: `Basis.z`_
    #[doc(alias = "z")]
    #[must_use]
    pub fn col_c(&self) -> Vector3 {
        Vector3::new(self.rows[0].z, self.rows[1].z, self.rows[2].z)
    }

    /// Set the values of the third column of the matrix.
    pub fn set_col_c(&mut self, col: Vector3) {
        self.rows[0].z = col.x;
        self.rows[1].z = col.y;
        self.rows[2].z = col.z;
    }
}

impl Display for Basis {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Godot output:
        // [X: (1, 0, 0), Y: (0, 1, 0), Z: (0, 0, 1)]
        // Where X,Y,Z are the columns
        let [a, b, c] = self.to_cols();

        write!(f, "[a: {a}, b: {b}, c: {c}]")
    }
}

impl ApproxEq for Basis {
    /// Returns if this basis and `other` are approximately equal, by calling `is_equal_approx` on each row.
    fn approx_eq(&self, other: &Self) -> bool {
        Vector3::approx_eq(&self.rows[0], &other.rows[0])
            && Vector3::approx_eq(&self.rows[1], &other.rows[1])
            && Vector3::approx_eq(&self.rows[2], &other.rows[2])
    }
}

impl GlamConv for Basis {
    type Glam = RMat3;
}

impl GlamType for RMat3 {
    type Mapped = Basis;

    fn to_front(&self) -> Self::Mapped {
        Basis::from_rows_array(&self.to_cols_array()).transposed()
    }

    fn from_front(mapped: &Self::Mapped) -> Self {
        Self::from_cols_array(&mapped.to_rows_array()).transpose()
    }
}

#[cfg(not(feature = "double-precision"))]
impl GlamType for glam::Mat3A {
    type Mapped = Basis;

    fn to_front(&self) -> Self::Mapped {
        Basis::from_rows_array(&self.to_cols_array()).transposed()
    }

    fn from_front(mapped: &Self::Mapped) -> Self {
        Self::from_cols_array(&mapped.to_rows_array()).transpose()
    }
}

impl Default for Basis {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Mul for Basis {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        self.glam2(&rhs, |a, b| a * b)
    }
}

impl Mul<real> for Basis {
    type Output = Self;

    fn mul(mut self, rhs: real) -> Self::Output {
        self *= rhs;
        self
    }
}

impl MulAssign<real> for Basis {
    fn mul_assign(&mut self, rhs: real) {
        self.rows[0] *= rhs;
        self.rows[1] *= rhs;
        self.rows[2] *= rhs;
    }
}

impl Mul<Vector3> for Basis {
    type Output = Vector3;

    fn mul(self, rhs: Vector3) -> Self::Output {
        self.glam2(&rhs, |a, b| a * b)
    }
}

// SAFETY:
// This type is represented as `Self` in Godot, so `*mut Self` is sound.
unsafe impl GodotFfi for Basis {
    ffi_methods! { type sys::GDExtensionTypePtr = *mut Self; .. }
}

/// The ordering used to interpret a set of euler angles as extrinsic
/// rotations.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[repr(C)]
pub enum EulerOrder {
    XYZ = 0,
    XZY = 1,
    YXZ = 2,
    YZX = 3,
    ZXY = 4,
    ZYX = 5,
}

#[cfg(test)]
mod test {
    use crate::builtin::real_consts::{FRAC_PI_2, PI};

    use crate::assert_eq_approx;

    use super::*;

    fn deg_to_rad(rotation: Vector3) -> Vector3 {
        Vector3::new(
            rotation.x.to_radians(),
            rotation.y.to_radians(),
            rotation.z.to_radians(),
        )
    }

    // Translated from Godot
    fn test_rotation(deg_original_euler: Vector3, rot_order: EulerOrder) {
        // This test:
        // 1. Converts the rotation vector from deg to rad.
        // 2. Converts euler to basis.
        // 3. Converts the above basis back into euler.
        // 4. Converts the above euler into basis again.
        // 5. Compares the basis obtained in step 2 with the basis of step 4
        //
        // The conversion "basis to euler", done in the step 3, may be different from
        // the original euler, even if the final rotation are the same.
        // This happens because there are more ways to represents the same rotation,
        // both valid, using eulers.
        // For this reason is necessary to convert that euler back to basis and finally
        // compares it.
        //
        // In this way we can assert that both functions: basis to euler / euler to basis
        // are correct.

        // Euler to rotation
        let original_euler: Vector3 = deg_to_rad(deg_original_euler);
        let to_rotation: Basis = Basis::from_euler(rot_order, original_euler);

        // Euler from rotation
        let euler_from_rotation: Vector3 = to_rotation.to_euler(rot_order);
        let rotation_from_computed_euler: Basis = Basis::from_euler(rot_order, euler_from_rotation);

        let res: Basis = to_rotation.inverse() * rotation_from_computed_euler;
        assert!(
            (res.col_a() - Vector3::RIGHT).length() <= 0.1,
            "Fail due to X {} with {deg_original_euler} using {rot_order:?}",
            res.col_a()
        );
        assert!(
            (res.col_b() - Vector3::UP).length() <= 0.1,
            "Fail due to Y {} with {deg_original_euler} using {rot_order:?}",
            res.col_b()
        );
        assert!(
            (res.col_c() - Vector3::BACK).length() <= 0.1,
            "Fail due to Z {} with {deg_original_euler} using {rot_order:?}",
            res.col_c()
        );

        // Double check `to_rotation` decomposing with XYZ rotation order.
        let euler_xyz_from_rotation: Vector3 = to_rotation.to_euler(EulerOrder::XYZ);
        let rotation_from_xyz_computed_euler: Basis =
            Basis::from_euler(EulerOrder::XYZ, euler_xyz_from_rotation);

        let res = to_rotation.inverse() * rotation_from_xyz_computed_euler;

        assert!(
        (res.col_a() - Vector3::new(1.0, 0.0, 0.0)).length() <= 0.1,
        "Double check with XYZ rot order failed, due to X {} with {deg_original_euler} using {rot_order:?}",
        res.col_a(),
    );
        assert!(
        (res.col_b() - Vector3::new(0.0, 1.0, 0.0)).length() <= 0.1,
        "Double check with XYZ rot order failed, due to Y {} with {deg_original_euler} using {rot_order:?}",
        res.col_b(),
    );
        assert!(
        (res.col_c() - Vector3::new(0.0, 0.0, 1.0)).length() <= 0.1,
        "Double check with XYZ rot order failed, due to Z {} with {deg_original_euler} using {rot_order:?}",
        res.col_c(),
    );
    }

    #[test]
    fn consts_behavior_correct() {
        let v = Vector3::new(1.0, 2.0, 3.0);

        assert_eq_approx!(Basis::IDENTITY * v, v);
        assert_eq_approx!(Basis::FLIP_X * v, Vector3::new(-v.x, v.y, v.z),);
        assert_eq_approx!(Basis::FLIP_Y * v, Vector3::new(v.x, -v.y, v.z),);
        assert_eq_approx!(Basis::FLIP_Z * v, Vector3::new(v.x, v.y, -v.z),);
    }

    #[test]
    fn basic_rotation_correct() {
        assert_eq_approx!(
            Basis::from_axis_angle(Vector3::FORWARD, 0.0) * Vector3::RIGHT,
            Vector3::RIGHT,
        );
        assert_eq_approx!(
            Basis::from_axis_angle(Vector3::FORWARD, FRAC_PI_2) * Vector3::RIGHT,
            Vector3::DOWN,
        );
        assert_eq_approx!(
            Basis::from_axis_angle(Vector3::FORWARD, PI) * Vector3::RIGHT,
            Vector3::LEFT,
        );
        assert_eq_approx!(
            Basis::from_axis_angle(Vector3::FORWARD, PI + FRAC_PI_2) * Vector3::RIGHT,
            Vector3::UP,
        );
    }

    // Translated from Godot
    #[test]
    fn basis_euler_conversions() {
        let euler_order_to_test: Vec<EulerOrder> = vec![
            EulerOrder::XYZ,
            EulerOrder::XZY,
            EulerOrder::YZX,
            EulerOrder::YXZ,
            EulerOrder::ZXY,
            EulerOrder::ZYX,
        ];

        let vectors_to_test: Vec<Vector3> = vec![
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(0.5, 0.5, 0.5),
            Vector3::new(-0.5, -0.5, -0.5),
            Vector3::new(40.0, 40.0, 40.0),
            Vector3::new(-40.0, -40.0, -40.0),
            Vector3::new(0.0, 0.0, -90.0),
            Vector3::new(0.0, -90.0, 0.0),
            Vector3::new(-90.0, 0.0, 0.0),
            Vector3::new(0.0, 0.0, 90.0),
            Vector3::new(0.0, 90.0, 0.0),
            Vector3::new(90.0, 0.0, 0.0),
            Vector3::new(0.0, 0.0, -30.0),
            Vector3::new(0.0, -30.0, 0.0),
            Vector3::new(-30.0, 0.0, 0.0),
            Vector3::new(0.0, 0.0, 30.0),
            Vector3::new(0.0, 30.0, 0.0),
            Vector3::new(30.0, 0.0, 0.0),
            Vector3::new(0.5, 50.0, 20.0),
            Vector3::new(-0.5, -50.0, -20.0),
            Vector3::new(0.5, 0.0, 90.0),
            Vector3::new(0.5, 0.0, -90.0),
            Vector3::new(360.0, 360.0, 360.0),
            Vector3::new(-360.0, -360.0, -360.0),
            Vector3::new(-90.0, 60.0, -90.0),
            Vector3::new(90.0, 60.0, -90.0),
            Vector3::new(90.0, -60.0, -90.0),
            Vector3::new(-90.0, -60.0, -90.0),
            Vector3::new(-90.0, 60.0, 90.0),
            Vector3::new(90.0, 60.0, 90.0),
            Vector3::new(90.0, -60.0, 90.0),
            Vector3::new(-90.0, -60.0, 90.0),
            Vector3::new(60.0, 90.0, -40.0),
            Vector3::new(60.0, -90.0, -40.0),
            Vector3::new(-60.0, -90.0, -40.0),
            Vector3::new(-60.0, 90.0, 40.0),
            Vector3::new(60.0, 90.0, 40.0),
            Vector3::new(60.0, -90.0, 40.0),
            Vector3::new(-60.0, -90.0, 40.0),
            Vector3::new(-90.0, 90.0, -90.0),
            Vector3::new(90.0, 90.0, -90.0),
            Vector3::new(90.0, -90.0, -90.0),
            Vector3::new(-90.0, -90.0, -90.0),
            Vector3::new(-90.0, 90.0, 90.0),
            Vector3::new(90.0, 90.0, 90.0),
            Vector3::new(90.0, -90.0, 90.0),
            Vector3::new(20.0, 150.0, 30.0),
            Vector3::new(20.0, -150.0, 30.0),
            Vector3::new(-120.0, -150.0, 30.0),
            Vector3::new(-120.0, -150.0, -130.0),
            Vector3::new(120.0, -150.0, -130.0),
            Vector3::new(120.0, 150.0, -130.0),
            Vector3::new(120.0, 150.0, 130.0),
        ];

        for order in euler_order_to_test.iter() {
            for vector in vectors_to_test.iter() {
                test_rotation(*vector, *order);
            }
        }
    }

    // Translated from Godot
    #[test]
    fn basis_finite_number_test() {
        let x: Vector3 = Vector3::new(0.0, 1.0, 2.0);
        let infinite: Vector3 = Vector3::new(real::NAN, real::NAN, real::NAN);

        assert!(
            Basis::from_cols(x, x, x).is_finite(),
            "Basis with all components finite should be finite"
        );

        assert!(
            !Basis::from_cols(infinite, x, x).is_finite(),
            "Basis with one component infinite should not be finite."
        );
        assert!(
            !Basis::from_cols(x, infinite, x).is_finite(),
            "Basis with one component infinite should not be finite."
        );
        assert!(
            !Basis::from_cols(x, x, infinite).is_finite(),
            "Basis with one component infinite should not be finite."
        );

        assert!(
            !Basis::from_cols(infinite, infinite, x).is_finite(),
            "Basis with two components infinite should not be finite."
        );
        assert!(
            !Basis::from_cols(infinite, x, infinite).is_finite(),
            "Basis with two components infinite should not be finite."
        );
        assert!(
            !Basis::from_cols(x, infinite, infinite).is_finite(),
            "Basis with two components infinite should not be finite."
        );

        assert!(
            !Basis::from_cols(infinite, infinite, infinite).is_finite(),
            "Basis with three components infinite should not be finite."
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_roundtrip() {
        let basis = Basis::IDENTITY;
        let expected_json = "{\"rows\":[{\"x\":1.0,\"y\":0.0,\"z\":0.0},{\"x\":0.0,\"y\":1.0,\"z\":0.0},{\"x\":0.0,\"y\":0.0,\"z\":1.0}]}";

        crate::builtin::test_utils::roundtrip(&basis, expected_json);
    }
}
