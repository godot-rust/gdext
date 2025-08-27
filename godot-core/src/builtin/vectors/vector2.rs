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
use crate::builtin::vectors::Vector2Axis;
use crate::builtin::{inner, real, RAffine2, RVec2, Vector2i};

/// Vector used for 2D math using floating point coordinates.
///
/// 2-element structure that can be used to represent continuous positions or directions in 2D space,
/// as well as any other pair of numeric values.
///
/// It uses floating-point coordinates of 32-bit precision, unlike the engine's `float` type which
/// is always 64-bit. The engine can be compiled with the option `precision=double` to use 64-bit
/// vectors; use the gdext library with the `double-precision` feature in that case.
///
#[doc = shared_vector_docs!()]
///
/// ### Navigation to `impl` blocks within this page
///
/// - [Constants](#constants)
/// - [Constructors and general vector functions](#constructors-and-general-vector-functions)
/// - [Specialized `Vector2` functions](#specialized-vector2-functions)
/// - [Float-specific functions](#float-specific-functions)
/// - [2D functions](#2d-functions)
/// - [2D and 3D functions](#2d-and-3d-functions)
/// - [Trait impls + operators](#trait-implementations)
///
/// # All vector types
///
/// | Dimension | Floating-point                       | Integer                                |
/// |-----------|--------------------------------------|----------------------------------------|
/// | 2D        | **`Vector2`**                        | [`Vector2i`][crate::builtin::Vector2i] |
/// | 3D        | [`Vector3`][crate::builtin::Vector3] | [`Vector3i`][crate::builtin::Vector3i] |
/// | 4D        | [`Vector4`][crate::builtin::Vector4] | [`Vector4i`][crate::builtin::Vector4i] |
///
/// <br>You can convert to `Vector2i` using [`cast_int()`][Self::cast_int].
///
/// # Godot docs
///
/// [`Vector2` (stable)](https://docs.godotengine.org/en/stable/classes/class_vector2.html)
#[derive(Default, Copy, Clone, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(C)]
pub struct Vector2 {
    /// The vector's X component.
    pub x: real,

    /// The vector's Y component.
    pub y: real,
}

/// # Constants
impl Vector2 {
    impl_vector_consts!(real);
    impl_float_vector_consts!();
    impl_vector2x_consts!(real);
}

impl_vector_fns!(Vector2, RVec2, real, (x, y));

/// # Specialized `Vector2` functions
impl Vector2 {
    /// Creates a unit Vector2 rotated to the given `angle` in radians. This is equivalent to doing `Vector2::new(angle.cos(), angle.sin())`
    /// or `Vector2::RIGHT.rotated(angle)`.
    ///
    /// ```no_run
    /// use godot::prelude::*;
    ///
    /// let a = Vector2::from_angle(0.0);                       // (1.0, 0.0)
    /// let b = Vector2::new(1.0, 0.0).angle();                 // 0.0
    /// let c = Vector2::from_angle(real_consts::PI / 2.0);     // (0.0, 1.0)
    /// ```
    #[inline]
    pub fn from_angle(angle: real) -> Self {
        Self::from_glam(RVec2::from_angle(angle))
    }

    /// Returns this vector's angle with respect to the positive X axis, or `(1.0, 0.0)` vector, in radians.
    ///
    /// For example, `Vector2::RIGHT.angle()` will return zero, `Vector2::DOWN.angle()` will return `PI / 2` (a quarter turn, or 90 degrees),
    ///  and `Vector2::new(1.0, -1.0).angle()` will return `-PI / 4` (a negative eighth turn, or -45 degrees).
    ///
    /// [Illustration of the returned angle.](https://raw.githubusercontent.com/godotengine/godot-docs/master/img/vector2_angle.png)
    ///
    /// Equivalent to the result of `y.atan2(x)`.
    #[inline]
    pub fn angle(self) -> real {
        self.y.atan2(self.x)
    }

    /// Returns the **signed** angle between `self` and the given vector, as radians in `[-π, +π]`.
    ///
    /// Note that behavior is different from 3D [`Vector3::angle_to()`] which returns the **unsigned** angle.
    #[inline]
    pub fn angle_to(self, to: Self) -> real {
        self.glam2(&to, |a, b| a.angle_to(b))
    }

    /// Returns the angle to the given vector, in radians.
    ///
    /// [Illustration of the returned angle.](https://raw.githubusercontent.com/godotengine/godot-docs/master/img/vector2_angle_to.png)
    #[inline]
    pub fn angle_to_point(self, to: Self) -> real {
        (to - self).angle()
    }

    /// Returns the 2D analog of the cross product for this vector and `with`.
    ///
    /// This is the signed area of the parallelogram formed by the two vectors. If the second vector is clockwise from the first vector,
    /// then the cross product is the positive area. If counter-clockwise, the cross product is the negative area. If the two vectors are
    /// parallel this returns zero, making it useful for testing if two vectors are parallel.
    ///
    /// Note: Cross product is not defined in 2D mathematically. This method embeds the 2D vectors in the XY plane of 3D space and uses
    /// their cross product's Z component as the analog.
    #[inline]
    pub fn cross(self, with: Self) -> real {
        self.to_glam().perp_dot(with.to_glam())
    }

    /// Returns a perpendicular vector rotated 90 degrees counter-clockwise compared to the original, with the same length.
    #[inline]
    pub fn orthogonal(self) -> Self {
        Self::new(self.y, -self.x)
    }

    /// Returns the result of rotating this vector by `angle` (in radians).
    #[inline]
    pub fn rotated(self, angle: real) -> Self {
        Self::from_glam(RAffine2::from_angle(angle).transform_vector2(self.to_glam()))
    }

    /// Returns the result of spherical linear interpolation between this vector and `to`, by amount `weight`.
    /// `weight` is on the range of 0.0 to 1.0, representing the amount of interpolation.
    ///
    /// This method also handles interpolating the lengths if the input vectors have different lengths.
    /// For the special case of one or both input vectors having zero length, this method behaves like [`Vector2::lerp`].
    #[inline]
    pub fn slerp(self, to: Self, weight: real) -> Self {
        let start_length_sq = self.length_squared();
        let end_length_sq = to.length_squared();
        if start_length_sq == 0.0 || end_length_sq == 0.0 {
            return self.lerp(to, weight);
        }
        let start_length = start_length_sq.sqrt();
        let result_length = real::lerp(start_length, end_length_sq.sqrt(), weight);
        let angle = self.angle_to(to);
        self.rotated(angle * weight) * (result_length / start_length)
    }

    #[doc(hidden)]
    #[inline]
    pub fn as_inner(&self) -> inner::InnerVector2<'_> {
        inner::InnerVector2::from_outer(self)
    }
}

impl_float_vector_fns!(Vector2, Vector2i, (x, y));
impl_vector2x_fns!(Vector2, Vector3, real);
impl_vector2_vector3_fns!(Vector2, (x, y));

impl_vector_operators!(Vector2, real, (x, y));

/// Formats the vector like Godot: `(x, y)`.
impl fmt::Display for Vector2 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

// SAFETY:
// This type is represented as `Self` in Godot, so `*mut Self` is sound.
unsafe impl GodotFfi for Vector2 {
    const VARIANT_TYPE: ExtVariantType = ExtVariantType::Concrete(sys::VariantType::VECTOR2);

    ffi_methods! { type sys::GDExtensionTypePtr = *mut Self; .. }
}

crate::meta::impl_godot_as_self!(Vector2: ByValue);

impl GlamConv for Vector2 {
    type Glam = RVec2;
}

impl GlamType for RVec2 {
    type Mapped = Vector2;

    fn to_front(&self) -> Self::Mapped {
        Vector2::new(self.x, self.y)
    }

    fn from_front(mapped: &Self::Mapped) -> Self {
        RVec2::new(mapped.x, mapped.y)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::assert_eq_approx;

    #[test]
    fn coord_min_max() {
        let a = Vector2::new(1.2, 3.4);
        let b = Vector2::new(0.1, 5.6);

        assert_eq_approx!(a.coord_min(b), Vector2::new(0.1, 3.4));
        assert_eq_approx!(a.coord_max(b), Vector2::new(1.2, 5.6));
    }

    #[test]
    fn sign() {
        let vector = Vector2::new(0.2, -0.5);
        assert_eq!(vector.sign(), Vector2::new(1., -1.));
        let vector = Vector2::new(0.1, 0.0);
        assert_eq!(vector.sign(), Vector2::new(1., 0.));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_roundtrip() {
        let vector = Vector2::default();
        let expected_json = "{\"x\":0.0,\"y\":0.0}";

        crate::builtin::test_utils::roundtrip(&vector, expected_json);
    }
}
