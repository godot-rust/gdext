/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

/// Convenience conversion between `real` and `f32`/`f64`.
///
/// Clippy often complains if you do `f as f64` when `f` is already an `f64`. This trait exists to make it easy to
/// convert between the different reals and floats without a lot of allowing clippy lints for your code.
pub trait RealConv {
    /// Cast this [`real`][type@real] to an [`f32`] using `as`.
    // Clippy complains that this is an `as_*` function, but it takes a `self`. Since this uses `as` internally, it makes much more sense for
    // it to be named `as_f32` rather than `to_f32`.
    #[allow(clippy::wrong_self_convention)]
    fn as_f32(self) -> f32;

    /// Cast this [`real`][type@real] to an [`f64`] using `as`.
    #[allow(clippy::wrong_self_convention)] // see above.
    fn as_f64(self) -> f64;

    /// Cast an [`f32`] to a [`real`][type@real] using `as`.
    fn from_f32(f: f32) -> Self;

    /// Cast an [`f64`] to a [`real`][type@real] using `as`.
    fn from_f64(f: f64) -> Self;
}

#[cfg(not(feature = "double-precision"))]
mod real_mod {
    /// Floating point type used for many structs and functions in Godot.
    ///
    /// This type is `f32` by default, and `f64` when the Cargo feature `double-precision` is enabled.
    ///
    /// This is not the `float` type in GDScript; that type is always 64-bits. Rather, many structs in Godot may use
    /// either 32-bit or 64-bit floats, for example [`Vector2`][crate::builtin::Vector2]. To convert between [`real`] and [`f32`] or
    /// [`f64`], see [`RealConv`](super::RealConv).
    ///
    /// See also the [Godot docs on float](https://docs.godotengine.org/en/stable/classes/class_float.html).
    // As this is a scalar value, we will use a non-standard type name.
    #[allow(non_camel_case_types)]
    pub type real = f32;

    impl super::RealConv for real {
        #[inline]
        fn as_f32(self) -> f32 {
            self
        }

        #[inline]
        fn as_f64(self) -> f64 {
            self as f64
        }

        #[inline]
        fn from_f32(f: f32) -> Self {
            f
        }

        #[inline]
        fn from_f64(f: f64) -> Self {
            f as f32
        }
    }

    /// Re-export of [`std::f32::consts`] or [`std::f64::consts`], depending on precision config.
    pub mod real_consts {
        pub use std::f32::consts::*;
    }

    /// A 2-dimensional vector from [`glam`]. Using a floating-point format compatible with [`real`].
    pub type RVec2 = glam::Vec2;
    /// A 3-dimensional vector from [`glam`]. Using a floating-point format compatible with [`real`].
    pub type RVec3 = glam::Vec3;
    /// A 4-dimensional vector from [`glam`]. Using a floating-point format compatible with [`real`].
    pub type RVec4 = glam::Vec4;

    /// A 2x2 column-major matrix from [`glam`]. Using a floating-point format compatible with [`real`].
    pub type RMat2 = glam::Mat2;
    /// A 3x3 column-major matrix from [`glam`]. Using a floating-point format compatible with [`real`].
    pub type RMat3 = glam::Mat3;
    /// A 4x4 column-major matrix from [`glam`]. Using a floating-point format compatible with [`real`].
    pub type RMat4 = glam::Mat4;

    /// A matrix from [`glam`] quaternion representing an orientation. Using a floating-point format compatible
    /// with [`real`].
    pub type RQuat = glam::Quat;

    /// A 2D affine transform from [`glam`], which can represent translation, rotation, scaling and
    /// shear. Using a floating-point format compatible with [`real`].
    pub type RAffine2 = glam::Affine2;
    /// A 3D affine transform from [`glam`], which can represent translation, rotation, scaling and
    /// shear. Using a floating-point format compatible with [`real`].
    pub type RAffine3 = glam::Affine3A;
}

#[cfg(feature = "double-precision")]
mod real_mod {
    /// Floating point type used for many structs and functions in Godot.
    ///
    /// This type is `f32` by default, and `f64` when the Cargo feature `double-precision` is enabled.
    ///
    /// This is not the `float` type in GDScript; that type is always 64-bits. Rather, many structs in Godot may use
    /// either 32-bit or 64-bit floats, for example [`Vector2`](super::Vector2). To convert between [`real`] and [`f32`] or
    /// [`f64`], see [`RealConv`](super::RealConv).
    ///
    /// See also the [Godot docs on float](https://docs.godotengine.org/en/stable/classes/class_float.html).
    // As this is a scalar value, we will use a non-standard type name.
    #[allow(non_camel_case_types)]
    pub type real = f64;

    impl super::RealConv for real {
        #[inline]
        fn as_f32(self) -> f32 {
            self as f32
        }

        #[inline]
        fn as_f64(self) -> f64 {
            self
        }

        #[inline]
        fn from_f32(f: f32) -> Self {
            f as f64
        }

        #[inline]
        fn from_f64(f: f64) -> Self {
            f
        }
    }

    /// Re-export of [`std::f32::consts`] or [`std::f64::consts`], depending on precision config.
    pub mod real_consts {
        pub use std::f64::consts::*;
    }

    /// A 2-dimensional vector from [`glam`]. Using a floating-point format compatible with [`real`].
    pub type RVec2 = glam::DVec2;
    /// A 3-dimensional vector from [`glam`]. Using a floating-point format compatible with [`real`].
    pub type RVec3 = glam::DVec3;
    /// A 4-dimensional vector from [`glam`]. Using a floating-point format compatible with [`real`].
    pub type RVec4 = glam::DVec4;

    /// A 2x2 column-major matrix from [`glam`]. Using a floating-point format compatible with [`real`].
    pub type RMat2 = glam::DMat2;
    /// A 3x3 column-major matrix from [`glam`]. Using a floating-point format compatible with [`real`].
    pub type RMat3 = glam::DMat3;
    /// A 4x4 column-major matrix from [`glam`]. Using a floating-point format compatible with [`real`].
    pub type RMat4 = glam::DMat4;

    /// A matrix from [`glam`] quaternion representing an orientation. Using a floating-point format
    /// compatible with [`real`].
    pub type RQuat = glam::DQuat;

    /// A 2D affine transform from [`glam`], which can represent translation, rotation, scaling and
    /// shear. Using a floating-point format compatible with [`real`].
    pub type RAffine2 = glam::DAffine2;
    /// A 3D affine transform from [`glam`], which can represent translation, rotation, scaling and
    /// shear. Using a floating-point format compatible with [`real`].
    pub type RAffine3 = glam::DAffine3;
}

// Public symbols (note that macro `real!` is re-exported in `lib.rs`)
pub use real_mod::{real, real_consts};

// ----------------------------------------------------------------------------------------------------------------------------------------------

// Internal re-exports
pub(crate) use real_mod::*;

/// A macro to coerce float-literals into the [`real`] type.
///
/// Mainly used where you'd normally use a suffix to specify the type, such as `115.0f32`.
///
/// # Examples
///
/// Rust is not able to infer the `self` type of this call to `to_radians`:
/// ```compile_fail
/// use godot::builtin::real;
///
/// let radians: real = 115.0.to_radians();
/// ```
/// But we cannot add a suffix to the literal, since it may be either `f32` or
/// `f64` depending on the context. So instead we use our macro:
/// ```
/// use godot::builtin::real;
///
/// let radians: real = real!(115.0).to_radians();
/// ```
#[macro_export]
macro_rules! real {
    ($f:literal) => {{
        let f: $crate::builtin::real = $f;
        f
    }};
}

/// Array of [`real`]s.
///
/// The expression has type `[real; N]` where `N` is the number of elements in the array.
///
/// # Example
/// ```
/// use godot_core::builtin::{real, reals};
///
/// let arr = reals![1.0, 2.0, 3.0];
/// assert_eq!(arr[1], real!(2.0));
/// ```
#[macro_export]
macro_rules! reals {
    ($($f:literal),* $(,)?) => {{
        let arr = [$($crate::real!($f)),*];
        arr
    }};
}
