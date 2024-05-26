/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

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
        $crate::builtin::SwizzleToVector::swizzle_to_vector((expr.$a, expr.$b))
    }};
    ($vec:expr => $a:ident, $b:ident, $c:ident) => {{
        let expr = $vec;
        $crate::builtin::SwizzleToVector::swizzle_to_vector((expr.$a, expr.$b, expr.$c))
    }};
    ($vec:expr => $a:ident, $b:ident, $c:ident, $d:ident) => {{
        let expr = $vec;
        $crate::builtin::SwizzleToVector::swizzle_to_vector((expr.$a, expr.$b, expr.$c, expr.$d))
    }};
}

/// Trait that allows conversion from tuples to vectors.
///
/// Is implemented instead of `From`/`Into` because it provides type inference.
/// Used for swizzle implementation.
#[doc(hidden)]
pub trait SwizzleToVector: Sized {
    type Output;
    fn swizzle_to_vector(self) -> Self::Output;
}
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
