/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::builtin::meta::{ConvertError, FromGodot, FromGodotError, GodotConvert, ToGodot};
use crate::builtin::{real, Vector2, Vector2i, Vector3, Vector3i, Vector4, Vector4i};
use crate::obj::EngineEnum;

/// Access vector components in different order.
///
/// Allows to rearrange components, as well as to create higher- or lower-order vectors.
///
/// This macro supports all vector types (2D, 3D, 4D; both integer and float). The resulting vector
/// type is deduced from the number and types of components.
///
/// To repeat a single component, check out the `splat` method on specific vector types.
///
/// # Examples
///
/// Reorder or duplicate fields:
/// ```
/// use godot::prelude::*;
///
/// let vec3 = Vector3i::new(1, 2, 3);
/// let xzx = swizzle!(vec3 => x, z, x); // Vector3i
///
/// assert_eq!(xzx, Vector3i::new(1, 3, 1));
/// ```
///
/// Create lower-order vector:
/// ```
/// # use godot::prelude::*;
/// let vec4 = Vector4::new(1.0, 2.0, 3.0, 4.0);
/// let yw = swizzle!(vec4 => y, w); // Vector2
///
/// assert_eq!(yw, Vector2::new(2.0, 4.0));
/// ```
///
/// Create higher-order vector:
/// ```
/// # use godot::prelude::*;
/// let vec3 = Vector3i::new(1, 2, 3);
/// let xyyz = swizzle!(vec3 => x, y, y, z); // Vector4i
///
/// assert_eq!(xyyz, Vector4i::new(1, 2, 2, 3));
/// ```
#[macro_export]
macro_rules! swizzle {
    ($vec:expr => $a:ident, $b:ident) => {{
        let expr = $vec;
        $crate::builtin::ToVector::to_vector((expr.$a, expr.$b))
    }};
    ($vec:expr => $a:ident, $b:ident, $c:ident) => {{
        let expr = $vec;
        $crate::builtin::ToVector::to_vector((expr.$a, expr.$b, expr.$c))
    }};
    ($vec:expr => $a:ident, $b:ident, $c:ident, $d:ident) => {{
        let expr = $vec;
        $crate::builtin::ToVector::to_vector((expr.$a, expr.$b, expr.$c, expr.$d))
    }};
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Trait that allows conversion from tuples to vectors.
///
/// Is implemented instead of `From`/`Into` because it provides type inference.
pub trait ToVector: Sized {
    type Output;
    fn to_vector(self) -> Self::Output;
}

/// Enumerates the axes in a [`Vector2`].
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
#[repr(i32)]
pub enum Vector2Axis {
    /// The X axis.
    X,

    /// The Y axis.
    Y,
}

impl EngineEnum for Vector2Axis {
    fn try_from_ord(ord: i32) -> Option<Self> {
        match ord {
            0 => Some(Self::X),
            1 => Some(Self::Y),
            _ => None,
        }
    }

    fn ord(self) -> i32 {
        self as i32
    }
}

impl GodotConvert for Vector2Axis {
    type Via = i32;
}

impl ToGodot for Vector2Axis {
    fn to_godot(&self) -> Self::Via {
        self.ord()
    }
}

impl FromGodot for Vector2Axis {
    fn try_from_godot(via: Self::Via) -> Result<Self, ConvertError> {
        Self::try_from_ord(via).ok_or_else(|| FromGodotError::InvalidEnum.into_error(via))
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Enumerates the axes in a [`Vector3`].
// TODO auto-generate this, alongside all the other builtin type's enums
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
#[repr(i32)]
pub enum Vector3Axis {
    /// The X axis.
    X,

    /// The Y axis.
    Y,

    /// The Z axis.
    Z,
}

impl EngineEnum for Vector3Axis {
    fn try_from_ord(ord: i32) -> Option<Self> {
        match ord {
            0 => Some(Self::X),
            1 => Some(Self::Y),
            2 => Some(Self::Z),
            _ => None,
        }
    }

    fn ord(self) -> i32 {
        self as i32
    }
}

impl GodotConvert for Vector3Axis {
    type Via = i32;
}

impl ToGodot for Vector3Axis {
    fn to_godot(&self) -> Self::Via {
        self.ord()
    }
}

impl FromGodot for Vector3Axis {
    fn try_from_godot(via: Self::Via) -> Result<Self, ConvertError> {
        Self::try_from_ord(via).ok_or_else(|| FromGodotError::InvalidEnum.into_error(via))
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Enumerates the axes in a [`Vector4`].
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
#[repr(i32)]
pub enum Vector4Axis {
    /// The X axis.
    X,

    /// The Y axis.
    Y,

    /// The Z axis.
    Z,

    /// The W axis.
    W,
}

impl EngineEnum for Vector4Axis {
    fn try_from_ord(ord: i32) -> Option<Self> {
        match ord {
            0 => Some(Self::X),
            1 => Some(Self::Y),
            2 => Some(Self::Z),
            3 => Some(Self::W),
            _ => None,
        }
    }

    fn ord(self) -> i32 {
        self as i32
    }
}

impl GodotConvert for Vector4Axis {
    type Via = i32;
}

impl ToGodot for Vector4Axis {
    fn to_godot(&self) -> Self::Via {
        self.ord()
    }
}

impl FromGodot for Vector4Axis {
    fn try_from_godot(via: Self::Via) -> Result<Self, ConvertError> {
        Self::try_from_ord(via).ok_or_else(|| FromGodotError::InvalidEnum.into_error(via))
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

impl_vector_index!(Vector2, real, (x, y), Vector2Axis, (X, Y));
impl_vector_index!(Vector2i, i32, (x, y), Vector2Axis, (X, Y));

impl_vector_index!(Vector3, real, (x, y, z), Vector3Axis, (X, Y, Z));
impl_vector_index!(Vector3i, i32, (x, y, z), Vector3Axis, (X, Y, Z));

impl_vector_index!(Vector4, real, (x, y, z, w), Vector4Axis, (X, Y, Z, W));
impl_vector_index!(Vector4i, i32, (x, y, z, w), Vector4Axis, (X, Y, Z, W));

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[cfg(test)]
mod test {
    use crate::assert_eq_approx;
    use crate::builtin::*;

    #[test]
    fn test_vector_swizzle() {
        // * VectorN swizzle
        let vector2 = Vector2::new(1.0, 2.0);
        let vector3 = Vector3::new(1.0, 2.0, 3.0);
        let vector4 = Vector4::new(1.0, 2.0, 3.0, 4.0);

        // VectorN to Vector2
        let vec2swiz2: Vector2 = swizzle!(vector2 => y, x);
        let vec3swiz2: Vector2 = swizzle!(vector3 => y, x);
        let vec4swiz2: Vector2 = swizzle!(vector4 => y, x);
        assert_eq_approx!(vec2swiz2, Vector2::new(2.0, 1.0));
        assert_eq_approx!(vec3swiz2, Vector2::new(2.0, 1.0));
        assert_eq_approx!(vec4swiz2, Vector2::new(2.0, 1.0));

        // VectorN to Vector3
        let vec2swiz3: Vector3 = swizzle!(vector2 => y, x, x);
        let vec3swiz3: Vector3 = swizzle!(vector3 => y, x, z);
        let vec4swiz3: Vector3 = swizzle!(vector4 => y, x, z);
        assert_eq_approx!(vec2swiz3, Vector3::new(2.0, 1.0, 1.0),);
        assert_eq_approx!(vec3swiz3, Vector3::new(2.0, 1.0, 3.0),);
        assert_eq_approx!(vec4swiz3, Vector3::new(2.0, 1.0, 3.0),);

        // VectorN to Vector4
        let vec2swiz4: Vector4 = swizzle!(vector2 => y, x, x, y);
        let vec3swiz4: Vector4 = swizzle!(vector3 => y, x, z, y);
        let vec4swiz4: Vector4 = swizzle!(vector4 => y, x, z, w);
        assert_eq_approx!(vec2swiz4, Vector4::new(2.0, 1.0, 1.0, 2.0),);
        assert_eq_approx!(vec3swiz4, Vector4::new(2.0, 1.0, 3.0, 2.0),);
        assert_eq_approx!(vec4swiz4, Vector4::new(2.0, 1.0, 3.0, 4.0),);

        // * VectorNi swizzle
        let vector2i = Vector2i::new(1, 2);
        let vector3i = Vector3i::new(1, 2, 3);
        let vector4i = Vector4i::new(1, 2, 3, 4);

        // VectorNi to Vector2i
        assert_eq!(Vector2i::new(2, 1), swizzle!(vector2i => y, x));
        assert_eq!(swizzle!(vector3i => y, x), Vector2i::new(2, 1));
        assert_eq!(swizzle!(vector4i => y, x), Vector2i::new(2, 1));

        // VectorNi to Vector3i
        assert_eq!(swizzle!(vector2i => y, x, x), Vector3i::new(2, 1, 1));
        assert_eq!(swizzle!(vector3i => y, x, z), Vector3i::new(2, 1, 3));
        assert_eq!(swizzle!(vector4i => y, x, z), Vector3i::new(2, 1, 3));

        // VectorNi to Vector4i
        assert_eq!(swizzle!(vector2i => y, x, x, y), Vector4i::new(2, 1, 1, 2));
        assert_eq!(swizzle!(vector3i => y, x, z, y), Vector4i::new(2, 1, 3, 2));
        assert_eq!(swizzle!(vector4i => y, x, z, w), Vector4i::new(2, 1, 3, 4));
    }
}
