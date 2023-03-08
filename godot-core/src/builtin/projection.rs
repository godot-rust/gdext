/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
use std::ops::*;

use godot_ffi as sys;
use sys::{ffi_methods, GodotFfi};

use super::glam_helpers::{GlamConv, GlamType};
use super::{inner::InnerProjection, Plane, Transform3D, Vector2, Vector4};
use super::{real, RMat4, RealConv};

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
        InnerProjection::create_depth_correction(flip_y)
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
        InnerProjection::create_for_hmd(
            eye as i64,
            aspect.as_f64(),
            intraocular_dist.as_f64(),
            display_width.as_f64(),
            display_to_lens.as_f64(),
            oversample.as_f64(),
            near.as_f64(),
            far.as_f64(),
        )
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
        InnerProjection::create_frustum(
            left.as_f64(),
            right.as_f64(),
            bottom.as_f64(),
            top.as_f64(),
            near.as_f64(),
            far.as_f64(),
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
        InnerProjection::create_frustum_aspect(
            size.as_f64(),
            aspect.as_f64(),
            offset,
            near.as_f64(),
            far.as_f64(),
            flip_fov,
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
        InnerProjection::create_orthogonal(
            left.as_f64(),
            right.as_f64(),
            bottom.as_f64(),
            top.as_f64(),
            near.as_f64(),
            far.as_f64(),
        )
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
        InnerProjection::create_orthogonal_aspect(
            size.as_f64(),
            aspect.as_f64(),
            near.as_f64(),
            far.as_f64(),
            flip_fov,
        )
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
        InnerProjection::create_perspective(
            fov_y.as_f64(),
            aspect.as_f64(),
            near.as_f64(),
            far.as_f64(),
            flip_fov,
        )
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
        InnerProjection::create_perspective_hmd(
            fov_y.as_f64(),
            aspect.as_f64(),
            near.as_f64(),
            far.as_f64(),
            flip_fov,
            eye as i64,
            intraocular_dist.as_f64(),
            convergence_dist.as_f64(),
        )
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
        self.as_inner().is_orthogonal()
    }

    /// Returns a Projection with the X and Y values from the given [`Vector2`]
    /// added to the first and second values of the final column respectively.
    ///
    /// _Godot equivalent: Projection.jitter_offseted()_
    #[must_use]
    pub fn jitter_offset(&self, offset: Vector2) -> Self {
        self.as_inner().jitter_offseted(offset)
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

impl GodotFfi for Projection {
    ffi_methods! { type sys::GDExtensionTypePtr = *mut Self; .. }
}

/// A projections clipping plane.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(C)]
pub enum ProjectionEye {
    Left = 1,
    Right = 2,
}
