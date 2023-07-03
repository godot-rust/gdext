/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot_ffi as sys;
use sys::{ffi_methods, GodotFfi};

use crate::builtin::inner::InnerProjection;
use crate::builtin::math::{ApproxEq, GlamConv, GlamType};
use crate::builtin::{real, Plane, RMat4, RealConv, Transform3D, Vector2, Vector4, Vector4Axis};

use std::ops::Mul;

/// A 4x4 matrix used for 3D projective transformations. It can represent
/// transformations such as translation, rotation, scaling, shearing, and
/// perspective division. It consists of four Vector4 columns.
///
/// For purely linear transformations (translation, rotation, and scale), it is
/// recommended to use Transform3D, as it is more performant and has a lower
/// memory footprint.
///
/// Used internally as Camera3D's projection matrix.
///
/// Note: The current implementation largely makes calls to godot for its
/// methods and as such are not as performant as other types.
#[derive(Copy, Clone, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(C)]
pub struct Projection {
    /// The columns of the projection matrix.
    pub cols: [Vector4; 4],
}

impl Projection {
    /// A Projection with no transformation defined. When applied to other data
    /// structures, no transformation is performed.
    ///
    /// _Godot equivalent: Projection.IDENTITY_
    pub const IDENTITY: Self = Self::from_diagonal(1.0, 1.0, 1.0, 1.0);

    /// A Projection with all values initialized to 0. When applied to other
    /// data structures, they will be zeroed.
    ///
    /// _Godot equivalent: Projection.ZERO_
    pub const ZERO: Self = Self::from_diagonal(0.0, 0.0, 0.0, 0.0);

    /// Create a new projection from a list of column vectors.
    pub const fn new(cols: [Vector4; 4]) -> Self {
        Self { cols }
    }

    /// Create a diagonal matrix from the given values.
    pub const fn from_diagonal(x: real, y: real, z: real, w: real) -> Self {
        Self::from_cols(
            Vector4::new(x, 0.0, 0.0, 0.0),
            Vector4::new(0.0, y, 0.0, 0.0),
            Vector4::new(0.0, 0.0, z, 0.0),
            Vector4::new(0.0, 0.0, 0.0, w),
        )
    }

    /// Create a matrix from four column vectors.
    ///
    /// _Godot equivalent: Projection(Vector4 x_axis, Vector4 y_axis, Vector4 z_axis, Vector4 w_axis)_
    pub const fn from_cols(x: Vector4, y: Vector4, z: Vector4, w: Vector4) -> Self {
        Self { cols: [x, y, z, w] }
    }

    /// Creates a new Projection that projects positions from a depth range of
    /// -1 to 1 to one that ranges from 0 to 1, and flips the projected
    /// positions vertically, according to flip_y.
    ///
    /// _Godot equivalent: Projection.create_depth_correction()_
    pub fn create_depth_correction(flip_y: bool) -> Self {
        Self::from_cols(
            Vector4::new(1.0, 0.0, 0.0, 0.0),
            Vector4::new(0.0, if flip_y { -1.0 } else { 1.0 }, 0.0, 0.0),
            Vector4::new(0.0, 0.0, 0.5, 0.0),
            Vector4::new(0.0, 0.0, 0.5, 1.0),
        )
    }

    /// Creates a new Projection for projecting positions onto a head-mounted
    /// display with the given X:Y aspect ratio, distance between eyes, display
    /// width, distance to lens, oversampling factor, and depth clipping planes.
    ///
    /// _Godot equivalent: Projection.create_for_hmd()_
    #[allow(clippy::too_many_arguments)]
    pub fn create_for_hmd(
        eye: ProjectionEye,
        aspect: real,
        intraocular_dist: real,
        display_width: real,
        display_to_lens: real,
        oversample: real,
        near: real,
        far: real,
    ) -> Self {
        let mut f1 = (intraocular_dist * 0.5) / display_to_lens;
        let mut f2 = ((display_width - intraocular_dist) * 0.5) / display_to_lens;
        let f3 = ((display_width * 0.25 * oversample) / (display_to_lens * aspect)) * near;

        let add = (f1 + f2) * (oversample - 1.0) * 0.5;
        f1 = (f1 + add) * near;
        f2 = (f2 + add) * near;

        match eye {
            ProjectionEye::Left => Self::create_frustum(-f2, f1, -f3, f3, near, far),
            ProjectionEye::Right => Self::create_frustum(-f1, f2, -f3, f3, near, far),
        }
    }

    /// Creates a new Projection that projects positions in a frustum with the
    /// given clipping planes.
    ///
    /// _Godot equivalent: Projection.create_frustum()_
    pub fn create_frustum(
        left: real,
        right: real,
        bottom: real,
        top: real,
        near: real,
        far: real,
    ) -> Self {
        let dx = right - left;
        let dy = top - bottom;
        let dz = near - far;

        let x = 2.0 * near / dx;
        let y = 2.0 * near / dy;
        let a = (right + left) / dx;
        let b = (top + bottom) / dy;
        let c = (far + near) / dz;
        let d = 2.0 * near * far / dz;

        Self::from_cols(
            Vector4::new(x, 0.0, 0.0, 0.0),
            Vector4::new(0.0, y, 0.0, 0.0),
            Vector4::new(a, b, c, -1.0),
            Vector4::new(0.0, 0.0, d, 0.0),
        )
    }

    /// Creates a new Projection that projects positions in a frustum with the
    /// given size, X:Y aspect ratio, offset, and clipping planes.
    ///
    /// `flip_fov` determines whether the projection's field of view is flipped
    /// over its diagonal.
    ///
    /// _Godot equivalent: Projection.create_frustum_aspect()_
    pub fn create_frustum_aspect(
        size: real,
        aspect: real,
        offset: Vector2,
        near: real,
        far: real,
        flip_fov: bool,
    ) -> Self {
        let (dx, dy) = if flip_fov {
            (size, size / aspect)
        } else {
            (size * aspect, size)
        };
        let dz = near - far;

        let x = 2.0 * near / dx;
        let y = 2.0 * near / dy;
        let a = 2.0 * offset.x / dx;
        let b = 2.0 * offset.y / dy;
        let c = (far + near) / dz;
        let d = 2.0 * near * far / dz;

        Self::from_cols(
            Vector4::new(x, 0.0, 0.0, 0.0),
            Vector4::new(0.0, y, 0.0, 0.0),
            Vector4::new(a, b, c, -1.0),
            Vector4::new(0.0, 0.0, d, 0.0),
        )
    }

    /// Creates a new Projection that projects positions using an orthogonal
    /// projection with the given clipping planes.
    ///
    /// _Godot equivalent: Projection.create_orthogonal()_
    pub fn create_orthogonal(
        left: real,
        right: real,
        bottom: real,
        top: real,
        near: real,
        far: real,
    ) -> Self {
        RMat4::orthographic_rh_gl(left, right, bottom, top, near, far).to_front()
    }

    /// Creates a new Projection that projects positions using an orthogonal
    /// projection with the given size, X:Y aspect ratio, and clipping planes.
    ///
    /// `flip_fov` determines whether the projection's field of view is flipped
    /// over its diagonal.
    ///
    /// _Godot equivalent: Projection.create_orthogonal_aspect()_
    pub fn create_orthogonal_aspect(
        size: real,
        aspect: real,
        near: real,
        far: real,
        flip_fov: bool,
    ) -> Self {
        let f = size / 2.0;

        if flip_fov {
            let fy = f / aspect;
            Self::create_orthogonal(-f, f, -fy, fy, near, far)
        } else {
            let fx = f * aspect;
            Self::create_orthogonal(-fx, fx, -f, f, near, far)
        }
    }

    /// Creates a new Projection that projects positions using a perspective
    /// projection with the given Y-axis field of view (in degrees), X:Y aspect
    /// ratio, and clipping planes
    ///
    /// `flip_fov` determines whether the projection's field of view is flipped
    /// over its diagonal.
    ///
    /// _Godot equivalent: Projection.create_perspective()_
    pub fn create_perspective(
        fov_y: real,
        aspect: real,
        near: real,
        far: real,
        flip_fov: bool,
    ) -> Self {
        let mut fov_y = fov_y.to_radians();
        if flip_fov {
            fov_y = ((fov_y * 0.5).tan() / aspect).atan() * 2.0;
        }

        RMat4::perspective_rh_gl(fov_y, aspect, near, far).to_front()
    }

    /// Creates a new Projection that projects positions using a perspective
    /// projection with the given Y-axis field of view (in degrees), X:Y aspect
    /// ratio, and clipping distances. The projection is adjusted for a
    /// head-mounted display with the given distance between eyes and distance
    /// to a point that can be focused on.
    ///
    /// `flip_fov` determines whether the projection's field of view is flipped
    /// over its diagonal.
    ///
    /// _Godot equivalent: Projection.create_perspective_hmd()_
    #[allow(clippy::too_many_arguments)]
    pub fn create_perspective_hmd(
        fov_y: real,
        aspect: real,
        near: real,
        far: real,
        flip_fov: bool,
        eye: ProjectionEye,
        intraocular_dist: real,
        convergence_dist: real,
    ) -> Self {
        let fov_y = fov_y.to_radians();

        let ymax = if flip_fov {
            (fov_y * 0.5).tan() / aspect
        } else {
            fov_y.tan()
        } * near;
        let xmax = ymax * aspect;
        let frustumshift = (intraocular_dist * near * 0.5) / convergence_dist;

        let (left, right, model_translation) = match eye {
            ProjectionEye::Left => (
                frustumshift - xmax,
                xmax + frustumshift,
                intraocular_dist / 2.0,
            ),
            ProjectionEye::Right => (
                -frustumshift - xmax,
                xmax - frustumshift,
                intraocular_dist / -2.0,
            ),
        };

        let mut ret = Self::create_frustum(left, right, -ymax, ymax, near, far);
        ret.cols[0] += ret.cols[3] * model_translation;
        ret
    }

    /// Return the determinant of the matrix.
    ///
    /// _Godot equivalent: Projection.determinant()_
    pub fn determinant(&self) -> real {
        self.glam(|mat| mat.determinant())
    }

    /// Returns a copy of this Projection with the signs of the values of the Y
    /// column flipped.
    ///
    /// _Godot equivalent: Projection.flipped_y()_
    pub fn flipped_y(self) -> Self {
        let [x, y, z, w] = self.cols;
        Self::from_cols(x, -y, z, w)
    }

    /// Returns the X:Y aspect ratio of this Projection's viewport.
    ///
    /// _Godot equivalent: Projection.get_aspect()_
    pub fn aspect(&self) -> real {
        real::from_f64(self.as_inner().get_aspect())
    }

    /// Returns the dimensions of the far clipping plane of the projection,
    /// divided by two.
    ///
    /// _Godot equivalent: Projection.get_far_plane_half_extents()_
    pub fn far_plane_half_extents(&self) -> Vector2 {
        self.as_inner().get_far_plane_half_extents()
    }

    /// Returns the horizontal field of view of the projection (in degrees).
    ///
    /// _Godot equivalent: Projection.get_fov()_
    pub fn fov(&self) -> real {
        real::from_f64(self.as_inner().get_fov())
    }

    /// Returns the vertical field of view of a projection (in degrees) which
    /// has the given horizontal field of view (in degrees) and aspect ratio.
    ///
    /// _Godot equivalent: Projection.get_fovy()_
    pub fn fovy_of(fov_x: real, aspect: real) -> real {
        real::from_f64(InnerProjection::get_fovy(fov_x.as_f64(), aspect.as_f64()))
    }

    /// Returns the factor by which the visible level of detail is scaled by
    /// this Projection.
    ///
    /// _Godot equivalent: Projection.get_lod_multiplier()_
    pub fn lod_multiplier(&self) -> real {
        real::from_f64(self.as_inner().get_lod_multiplier())
    }

    /// Returns the number of pixels with the given pixel width displayed per
    /// meter, after this Projection is applied.
    ///
    /// _Godot equivalent: Projection.get_pixels_per_meter()_
    pub fn pixels_per_meter(&self, pixel_width: i64) -> i64 {
        self.as_inner().get_pixels_per_meter(pixel_width)
    }

    /// Returns the clipping plane of this Projection whose index is given by
    /// plane.
    ///
    /// _Godot equivalent: Projection.get_projection_plane()_
    pub fn projection_plane(&self, plane: ProjectionPlane) -> Plane {
        self.as_inner().get_projection_plane(plane as i64)
    }

    /// Returns the dimensions of the viewport plane that this Projection
    /// projects positions onto, divided by two.
    ///
    /// _Godot equivalent: Projection.get_viewport_half_extents()_
    pub fn viewport_half_extents(&self) -> Vector2 {
        self.as_inner().get_viewport_half_extents()
    }

    /// Returns the distance for this Projection beyond which positions are
    /// clipped.
    ///
    /// _Godot equivalent: Projection.get_z_far()_
    pub fn z_far(&self) -> real {
        real::from_f64(self.as_inner().get_z_far())
    }

    /// Returns the distance for this Projection before which positions are
    /// clipped.
    ///
    /// _Godot equivalent: Projection.get_z_near()_
    pub fn z_near(&self) -> real {
        real::from_f64(self.as_inner().get_z_near())
    }

    /// Returns a Projection that performs the inverse of this Projection's
    /// projective transformation.
    ///
    /// _Godot equivalent: Projection.inverse()_
    pub fn inverse(self) -> Self {
        self.glam(|mat| mat.inverse())
    }

    /// Returns `true` if this Projection performs an orthogonal projection.
    ///
    /// _Godot equivalent: Projection.is_orthogonal()_
    pub fn is_orthogonal(&self) -> bool {
        self.cols[3].w == 1.0

        // TODO: Test the entire last row?
        // The argument is that W should not mixed with any other dimensions.
        // But if the only operation is projection and affine, it suffice
        // to check if input W is nullified (v33 is zero).
        // (Currently leave it as-is, matching Godot's implementation).
        // (self.cols[0].w == 0.0) && (self.cols[1].w == 0.0) && (self.cols[2] == 0.0) && (self.cols[3].w == 1.0)
    }

    /// Returns a Projection with the X and Y values from the given [`Vector2`]
    /// added to the first and second values of the final column respectively.
    ///
    /// _Godot equivalent: Projection.jitter_offseted()_
    #[must_use]
    pub fn jitter_offset(&self, offset: Vector2) -> Self {
        Self::from_cols(
            self.cols[0],
            self.cols[1],
            self.cols[2],
            self.cols[3] + Vector4::new(offset.x, offset.y, 0.0, 0.0),
        )
    }

    /// Returns a Projection with the near clipping distance adjusted to be
    /// `new_znear`.
    ///
    /// Note: The original Projection must be a perspective projection.
    ///
    /// _Godot equivalent: Projection.perspective_znear_adjusted()_
    pub fn perspective_znear_adjusted(&self, new_znear: real) -> Self {
        self.as_inner()
            .perspective_znear_adjusted(new_znear.as_f64())
    }

    #[doc(hidden)]
    pub(crate) fn as_inner(&self) -> InnerProjection {
        InnerProjection::from_outer(self)
    }
}

impl From<Transform3D> for Projection {
    fn from(trans: Transform3D) -> Self {
        trans.glam(RMat4::from)
    }
}

impl Default for Projection {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Mul for Projection {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        self.glam2(&rhs, |a, b| a * b)
    }
}

impl Mul<Vector4> for Projection {
    type Output = Vector4;

    fn mul(self, rhs: Vector4) -> Self::Output {
        self.glam2(&rhs, |m, v| m * v)
    }
}

impl ApproxEq for Projection {
    fn approx_eq(&self, other: &Self) -> bool {
        for i in 0..4 {
            let v = self.cols[i];
            let w = other.cols[i];

            if !v.x.approx_eq(&w.x)
                || !v.y.approx_eq(&w.y)
                || !v.z.approx_eq(&w.z)
                || !v.w.approx_eq(&w.w)
            {
                return false;
            }
        }
        true
    }
}

impl GlamType for RMat4 {
    type Mapped = Projection;

    fn to_front(&self) -> Self::Mapped {
        Projection::from_cols(
            self.x_axis.to_front(),
            self.y_axis.to_front(),
            self.z_axis.to_front(),
            self.w_axis.to_front(),
        )
    }

    fn from_front(mapped: &Self::Mapped) -> Self {
        Self::from_cols_array_2d(&mapped.cols.map(|v| v.to_glam().to_array()))
    }
}

impl GlamConv for Projection {
    type Glam = RMat4;
}

// SAFETY:
// This type is represented as `Self` in Godot, so `*mut Self` is sound.
unsafe impl GodotFfi for Projection {
    ffi_methods! { type sys::GDExtensionTypePtr = *mut Self; .. }
}

/// A projections clipping plane.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
#[repr(C)]
pub enum ProjectionPlane {
    Near = 0,
    Far = 1,
    Left = 2,
    Top = 3,
    Right = 4,
    Bottom = 5,
}

/// The eye to create a projection for, when creating a projection adjusted
/// for head-mounted displays.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
#[repr(C)]
pub enum ProjectionEye {
    Left = 1,
    Right = 2,
}

#[cfg(test)]
mod test {
    // TODO(bromeon): reduce code duplication

    #![allow(clippy::type_complexity, clippy::excessive_precision)]

    use crate::assert_eq_approx;

    use super::*;

    const EPSILON: real = 1e-6;

    fn is_matrix_equal_approx(a: &Projection, b: &RMat4) -> bool {
        a.to_glam().abs_diff_eq(*b, EPSILON)
    }

    /// Test that diagonals matrices has certain property.
    #[test]
    fn test_diagonals() {
        const DIAGONALS: [[real; 4]; 10] = [
            [1.0, 1.0, 1.0, 1.0],
            [2.0, 1.0, 2.0, 1.0],
            [3.0, 2.0, 1.0, 1.0],
            [-1.0, -1.0, 1.0, 1.0],
            [0.0, 0.0, 0.0, 0.0],
            [-2.0, -3.0, -4.0, -5.0],
            [0.0, 5.0, -10.0, 50.0],
            [-1.0, 0.0, 1.0, 100.0],
            [-15.0, -22.0, 0.0, 11.0],
            [-1.0, 3.0, 1.0, 0.0],
        ];

        for [x, y, z, w] in DIAGONALS {
            let proj = Projection::from_diagonal(x, y, z, w);
            assert_eq_approx!(
                proj,
                RMat4::from_cols_array(&[
                    x, 0.0, 0.0, 0.0, 0.0, y, 0.0, 0.0, 0.0, 0.0, z, 0.0, 0.0, 0.0, 0.0, w,
                ]),
                fn = is_matrix_equal_approx,
            );

            let det = x * y * z * w;
            assert_eq_approx!(proj.determinant(), det);
            if det.abs() > 1e-6 {
                assert_eq_approx!(
                    proj.inverse(),
                    RMat4::from_cols_array_2d(&[
                        [1.0 / x, 0.0, 0.0, 0.0],
                        [0.0, 1.0 / y, 0.0, 0.0],
                        [0.0, 0.0, 1.0 / z, 0.0],
                        [0.0, 0.0, 0.0, 1.0 / w],
                    ]),
                    fn = is_matrix_equal_approx,
                );
            }
        }
    }

    /// Test `create_orthogonal` method.
    /// All inputs and outputs are manually computed.
    #[test]
    fn test_orthogonal() {
        const TEST_DATA: [([real; 6], [[real; 4]; 4]); 6] = [
            (
                [-1.0, 1.0, -1.0, 1.0, -1.0, 1.0],
                [
                    [1.0, 0.0, 0.0, 0.0],
                    [0.0, 1.0, 0.0, 0.0],
                    [0.0, 0.0, -1.0, 0.0],
                    [0.0, 0.0, 0.0, 1.0],
                ],
            ),
            (
                [0.0, 1.0, 0.0, 1.0, 0.0, 1.0],
                [
                    [2.0, 0.0, 0.0, 0.0],
                    [0.0, 2.0, 0.0, 0.0],
                    [0.0, 0.0, -2.0, 0.0],
                    [-1.0, -1.0, -1.0, 1.0],
                ],
            ),
            (
                [-1.0, 1.0, -1.0, 1.0, 0.0, 1.0],
                [
                    [1.0, 0.0, 0.0, 0.0],
                    [0.0, 1.0, 0.0, 0.0],
                    [0.0, 0.0, -2.0, 0.0],
                    [0.0, 0.0, -1.0, 1.0],
                ],
            ),
            (
                [-10.0, 10.0, -10.0, 10.0, 0.0, 100.0],
                [
                    [0.1, 0.0, 0.0, 0.0],
                    [0.0, 0.1, 0.0, 0.0],
                    [0.0, 0.0, -0.02, 0.0],
                    [0.0, 0.0, -1.0, 1.0],
                ],
            ),
            (
                [-1.0, 1.0, -1.0, 1.0, 1.0, -1.0],
                [
                    [1.0, 0.0, 0.0, 0.0],
                    [0.0, 1.0, 0.0, 0.0],
                    [0.0, 0.0, 1.0, 0.0],
                    [0.0, 0.0, 0.0, 1.0],
                ],
            ),
            (
                [10.0, -10.0, 10.0, -10.0, -10.0, 10.0],
                [
                    [-0.1, 0.0, 0.0, 0.0],
                    [0.0, -0.1, 0.0, 0.0],
                    [0.0, 0.0, -0.1, 0.0],
                    [0.0, 0.0, 0.0, 1.0],
                ],
            ),
        ];

        for ([left, right, bottom, top, near, far], mat) in TEST_DATA {
            assert_eq_approx!(
                Projection::create_orthogonal(left, right, bottom, top, near, far),
                RMat4::from_cols_array_2d(&mat),
                fn = is_matrix_equal_approx,
                "orthogonal: left={left} right={right} bottom={bottom} top={top} near={near} far={far}",
            );
        }
    }

    /// Test `create_orthogonal_aspect` method.
    #[test]
    fn test_orthogonal_aspect() {
        const TEST_DATA: [((real, real, real, real, bool), [[real; 4]; 4]); 6] = [
            (
                (2.0, 1.0, 0.0, 1.0, false),
                [
                    [1.0, 0.0, 0.0, 0.0],
                    [0.0, 1.0, 0.0, 0.0],
                    [0.0, 0.0, -2.0, 0.0],
                    [0.0, 0.0, -1.0, 1.0],
                ],
            ),
            (
                (2.0, 1.0, 0.0, 1.0, true),
                [
                    [1.0, 0.0, 0.0, 0.0],
                    [0.0, 1.0, 0.0, 0.0],
                    [0.0, 0.0, -2.0, 0.0],
                    [0.0, 0.0, -1.0, 1.0],
                ],
            ),
            (
                (1.0, 2.0, 0.0, 100.0, false),
                [
                    [1.0, 0.0, 0.0, 0.0],
                    [0.0, 2.0, 0.0, 0.0],
                    [0.0, 0.0, -0.02, 0.0],
                    [0.0, 0.0, -1.0, 1.0],
                ],
            ),
            (
                (1.0, 2.0, 0.0, 100.0, true),
                [
                    [2.0, 0.0, 0.0, 0.0],
                    [0.0, 4.0, 0.0, 0.0],
                    [0.0, 0.0, -0.02, 0.0],
                    [0.0, 0.0, -1.0, 1.0],
                ],
            ),
            (
                (64.0, 9.0 / 16.0, 0.0, 100.0, false),
                [
                    [(1.0 / 32.0) * (16.0 / 9.0), 0.0, 0.0, 0.0],
                    [0.0, 1.0 / 32.0, 0.0, 0.0],
                    [0.0, 0.0, -0.02, 0.0],
                    [0.0, 0.0, -1.0, 1.0],
                ],
            ),
            (
                (64.0, 9.0 / 16.0, 0.0, 100.0, true),
                [
                    [1.0 / 32.0, 0.0, 0.0, 0.0],
                    [0.0, (1.0 / 32.0) * (9.0 / 16.0), 0.0, 0.0],
                    [0.0, 0.0, -0.02, 0.0],
                    [0.0, 0.0, -1.0, 1.0],
                ],
            ),
        ];

        for ((size, aspect, near, far, flip_fov), mat) in TEST_DATA {
            assert_eq_approx!(
                Projection::create_orthogonal_aspect(size, aspect, near, far, flip_fov),
                RMat4::from_cols_array_2d(&mat),
                fn = is_matrix_equal_approx,
                "orthogonal aspect: size={size} aspect={aspect} near={near} far={far} flip_fov={flip_fov}"
            );
        }
    }

    #[test]
    fn test_perspective() {
        const TEST_DATA: [((real, real, real, real, bool), [[real; 4]; 4]); 5] = [
            (
                (90.0, 1.0, 1.0, 2.0, false),
                [
                    [1.0, 0.0, 0.0, 0.0],
                    [0.0, 1.0, 0.0, 0.0],
                    [0.0, 0.0, -3.0, -1.0],
                    [0.0, 0.0, -4.0, 0.0],
                ],
            ),
            (
                (90.0, 1.0, 1.0, 2.0, true),
                [
                    [1.0, 0.0, 0.0, 0.0],
                    [0.0, 1.0, 0.0, 0.0],
                    [0.0, 0.0, -3.0, -1.0],
                    [0.0, 0.0, -4.0, 0.0],
                ],
            ),
            (
                (45.0, 1.0, 0.05, 100.0, false),
                [
                    [2.414213562373095, 0.0, 0.0, 0.0],
                    [0.0, 2.414213562373095, 0.0, 0.0],
                    [0.0, 0.0, -1.001000500250125, -1.0],
                    [0.0, 0.0, -0.10005002501250625, 0.0],
                ],
            ),
            (
                (90.0, 9.0 / 16.0, 1.0, 2.0, false),
                [
                    [16.0 / 9.0, 0.0, 0.0, 0.0],
                    [0.0, 1.0, 0.0, 0.0],
                    [0.0, 0.0, -3.0, -1.0],
                    [0.0, 0.0, -4.0, 0.0],
                ],
            ),
            (
                (90.0, 9.0 / 16.0, 1.0, 2.0, true),
                [
                    [1.0, 0.0, 0.0, 0.0],
                    [0.0, 9.0 / 16.0, 0.0, 0.0],
                    [0.0, 0.0, -3.0, -1.0],
                    [0.0, 0.0, -4.0, 0.0],
                ],
            ),
        ];

        for ((fov_y, aspect, near, far, flip_fov), mat) in TEST_DATA {
            assert_eq_approx!(
                Projection::create_perspective(fov_y, aspect, near, far, flip_fov),
                RMat4::from_cols_array_2d(&mat),
                fn = is_matrix_equal_approx,
                "perspective: fov_y={fov_y} aspect={aspect} near={near} far={far} flip_fov={flip_fov}"
            );
        }
    }

    #[test]
    fn test_frustum() {
        const TEST_DATA: [([real; 6], [[real; 4]; 4]); 3] = [
            (
                [-1.0, 1.0, -1.0, 1.0, 1.0, 2.0],
                [
                    [1.0, 0.0, 0.0, 0.0],
                    [0.0, 1.0, 0.0, 0.0],
                    [0.0, 0.0, -3.0, -1.0],
                    [0.0, 0.0, -4.0, 0.0],
                ],
            ),
            (
                [0.0, 1.0, 0.0, 1.0, 1.0, 2.0],
                [
                    [2.0, 0.0, 0.0, 0.0],
                    [0.0, 2.0, 0.0, 0.0],
                    [1.0, 1.0, -3.0, -1.0],
                    [0.0, 0.0, -4.0, 0.0],
                ],
            ),
            (
                [-0.1, 0.1, -0.025, 0.025, 0.05, 100.0],
                [
                    [0.5, 0.0, 0.0, 0.0],
                    [0.0, 2.0, 0.0, 0.0],
                    [0.0, 0.0, -1.001000500250125, -1.0],
                    [0.0, 0.0, -0.10005002501250625, 0.0],
                ],
            ),
        ];

        for ([left, right, bottom, top, near, far], mat) in TEST_DATA {
            assert_eq_approx!(
                Projection::create_frustum(left, right, bottom, top, near, far),
                RMat4::from_cols_array_2d(&mat),
                fn = is_matrix_equal_approx,
                "frustum: left={left} right={right} bottom={bottom} top={top} near={near} far={far}"
            );
        }
    }

    #[test]
    fn test_frustum_aspect() {
        const TEST_DATA: [((real, real, Vector2, real, real, bool), [[real; 4]; 4]); 4] = [
            (
                (2.0, 1.0, Vector2::ZERO, 1.0, 2.0, false),
                [
                    [1.0, 0.0, 0.0, 0.0],
                    [0.0, 1.0, 0.0, 0.0],
                    [0.0, 0.0, -3.0, -1.0],
                    [0.0, 0.0, -4.0, 0.0],
                ],
            ),
            (
                (2.0, 1.0, Vector2::ZERO, 1.0, 2.0, true),
                [
                    [1.0, 0.0, 0.0, 0.0],
                    [0.0, 1.0, 0.0, 0.0],
                    [0.0, 0.0, -3.0, -1.0],
                    [0.0, 0.0, -4.0, 0.0],
                ],
            ),
            (
                (1.0, 1.0, Vector2::new(0.5, 0.5), 1.0, 2.0, false),
                [
                    [2.0, 0.0, 0.0, 0.0],
                    [0.0, 2.0, 0.0, 0.0],
                    [1.0, 1.0, -3.0, -1.0],
                    [0.0, 0.0, -4.0, 0.0],
                ],
            ),
            (
                (0.05, 4.0, Vector2::ZERO, 0.05, 100.0, false),
                [
                    [0.5, 0.0, 0.0, 0.0],
                    [0.0, 2.0, 0.0, 0.0],
                    [0.0, 0.0, -1.001000500250125, -1.0],
                    [0.0, 0.0, -0.10005002501250625, 0.0],
                ],
            ),
        ];

        for ((size, aspect, offset, near, far, flip_fov), mat) in TEST_DATA {
            assert_eq_approx!(
                Projection::create_frustum_aspect(size, aspect, offset, near, far, flip_fov),
                RMat4::from_cols_array_2d(&mat),
                fn = is_matrix_equal_approx,
                "frustum aspect: size={size} aspect={aspect} offset=({0} {1}) near={near} far={far} flip_fov={flip_fov}",
                offset.x,
                offset.y,
            );
        }
    }

    // TODO: Test create_for_hmd, create_perspective_hmd

    #[test]
    fn test_is_orthogonal() {
        fn f(v: isize) -> real {
            (v as real) * 0.5 - 0.5
        }

        // Orthogonal
        for left_i in 0..20 {
            let left = f(left_i);
            for right in (left_i + 1..=20).map(f) {
                for bottom_i in 0..20 {
                    let bottom = f(bottom_i);
                    for top in (bottom_i + 1..=20).map(f) {
                        for near_i in 0..20 {
                            let near = f(near_i);
                            for far in (near_i + 1..=20).map(f) {
                                assert!(
                                    Projection::create_orthogonal(left, right, bottom, top, near, far).is_orthogonal(),
                                    "projection should be orthogonal: left={left} right={right} bottom={bottom} top={top} near={near} far={far}",
                                );
                            }
                        }
                    }
                }
            }
        }

        // Perspective
        for fov in (0..18).map(|v| (v as real) * 10.0) {
            for aspect_x in 1..=10 {
                for aspect_y in 1..=10 {
                    let aspect = (aspect_x as real) / (aspect_y as real);
                    for near_i in 1..10 {
                        let near = near_i as real;
                        for far in (near_i + 1..=20).map(|v| v as real) {
                            assert!(
                                !Projection::create_perspective(fov, aspect, near, far, false).is_orthogonal(),
                                "projection should be perspective: fov={fov} aspect={aspect} near={near} far={far}",
                            );
                        }
                    }
                }
            }
        }

        // Frustum
        for left_i in 0..20 {
            let left = f(left_i);
            for right in (left_i + 1..=20).map(f) {
                for bottom_i in 0..20 {
                    let bottom = f(bottom_i);
                    for top in (bottom_i + 1..=20).map(f) {
                        for near_i in 0..20 {
                            let near = (near_i as real) * 0.5;
                            for far in (near_i + 1..=20).map(|v| (v as real) * 0.5) {
                                assert!(
                                    !Projection::create_frustum(left, right, bottom, top, near, far).is_orthogonal(),
                                    "projection should be perspective: left={left} right={right} bottom={bottom} top={top} near={near} far={far}",
                                );
                            }
                        }
                    }
                }
            }
        }

        // Size, Aspect, Near, Far
        for size in (1..=10).map(|v| v as real) {
            for aspect_x in 1..=10 {
                for aspect_y in 1..=10 {
                    let aspect = (aspect_x as real) / (aspect_y as real);
                    for near_i in 1..10 {
                        let near = near_i as real;
                        for far in (near_i + 1..=20).map(|v| v as real) {
                            assert!(
                                Projection::create_orthogonal_aspect(size, aspect, near, far, false).is_orthogonal(),
                                "projection should be orthogonal: (size={size} aspect={aspect} near={near} far={far}",
                            );
                            assert!(
                                !Projection::create_frustum_aspect(size, aspect, Vector2::ZERO, near, far, false).is_orthogonal(),
                                "projection should be perspective: (size={size} aspect={aspect} near={near} far={far}",
                            );
                        }
                    }
                }
            }
        }
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_roundtrip() {
        let projection = Projection::IDENTITY;
        let expected_json = "{\"cols\":[{\"x\":1.0,\"y\":0.0,\"z\":0.0,\"w\":0.0},{\"x\":0.0,\"y\":1.0,\"z\":0.0,\"w\":0.0},{\"x\":0.0,\"y\":0.0,\"z\":1.0,\"w\":0.0},{\"x\":0.0,\"y\":0.0,\"z\":0.0,\"w\":1.0}]}";

        crate::builtin::test_utils::roundtrip(&projection, expected_json);
    }
}

impl std::fmt::Display for Projection {
    /// Formats `Projection` to match Godot's string representation.
    ///
    /// Example:
    /// ```
    /// use godot::prelude::*;
    /// let proj = Projection::new([
    ///     Vector4::new(1.0, 2.5, 1.0, 0.5),
    ///     Vector4::new(0.0, 1.5, 2.0, 0.5),
    ///     Vector4::new(0.0, 0.0, 3.0, 2.5),
    ///     Vector4::new(3.0, 1.0, 4.0, 1.5),
    /// ]);
    /// const FMT_RESULT: &str = r"
    /// 1, 0, 0, 3
    /// 2.5, 1.5, 0, 1
    /// 1, 2, 3, 4
    /// 0.5, 0.5, 2.5, 1.5
    /// ";
    /// assert_eq!(format!("{}", proj), FMT_RESULT);
    /// ```
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "\n{}, {}, {}, {}\n{}, {}, {}, {}\n{}, {}, {}, {}\n{}, {}, {}, {}\n",
            // first row
            self.cols[0][Vector4Axis::X],
            self.cols[1][Vector4Axis::X],
            self.cols[2][Vector4Axis::X],
            self.cols[3][Vector4Axis::X],
            // second row
            self.cols[0][Vector4Axis::Y],
            self.cols[1][Vector4Axis::Y],
            self.cols[2][Vector4Axis::Y],
            self.cols[3][Vector4Axis::Y],
            // third row
            self.cols[0][Vector4Axis::Z],
            self.cols[1][Vector4Axis::Z],
            self.cols[2][Vector4Axis::Z],
            self.cols[3][Vector4Axis::Z],
            // forth row
            self.cols[0][Vector4Axis::W],
            self.cols[1][Vector4Axis::W],
            self.cols[2][Vector4Axis::W],
            self.cols[3][Vector4Axis::W],
        )
    }
}
