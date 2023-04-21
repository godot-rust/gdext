/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

mod vector_macros;

pub mod vector2;
pub mod vector2i;
pub mod vector3;
pub mod vector3i;
pub mod vector4;
pub mod vector4i;

pub mod vector_utils;

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

        let vc2swiz2: Vector2 = swizzle!(vector2 => y, x);
        let vc3swiz2: Vector2 = swizzle!(vector3 => y, x);
        let vc4swiz2: Vector2 = swizzle!(vector4 => y, x);
        assert_eq_approx!(Vector2::new(2.0, 1.0), vc2swiz2, Vector2::is_equal_approx);
        assert_eq_approx!(Vector2::new(2.0, 1.0), vc3swiz2, Vector2::is_equal_approx);
        assert_eq_approx!(Vector2::new(2.0, 1.0), vc4swiz2, Vector2::is_equal_approx);

        // VectorN to Vector3

        let vc2swiz3: Vector3 = swizzle!(vector2 => y, x, x);
        let vc3swiz3: Vector3 = swizzle!(vector3 => y, x, z);
        let vc4swiz3: Vector3 = swizzle!(vector4 => y, x, z);
        assert_eq_approx!(
            Vector3::new(2.0, 1.0, 1.0),
            vc2swiz3,
            Vector3::is_equal_approx
        );
        assert_eq_approx!(
            Vector3::new(2.0, 1.0, 3.0),
            vc3swiz3,
            Vector3::is_equal_approx
        );
        assert_eq_approx!(
            Vector3::new(2.0, 1.0, 3.0),
            vc4swiz3,
            Vector3::is_equal_approx
        );

        // VectorN to Vector4

        let vc2swiz4: Vector4 = swizzle!(vector2 => y, x, x, y);
        let vc3swiz4: Vector4 = swizzle!(vector3 => y, x, z, y);
        let vc4swiz4: Vector4 = swizzle!(vector4 => y, x, z, w);
        assert_eq_approx!(
            Vector4::new(2.0, 1.0, 1.0, 2.0),
            vc2swiz4,
            Vector4::is_equal_approx
        );
        assert_eq_approx!(
            Vector4::new(2.0, 1.0, 3.0, 2.0),
            vc3swiz4,
            Vector4::is_equal_approx
        );
        assert_eq_approx!(
            Vector4::new(2.0, 1.0, 3.0, 4.0),
            vc4swiz4,
            Vector4::is_equal_approx
        );

        // * VectorNi swizzle
        let vector2i = Vector2i::new(1, 2);
        let vector3i = Vector3i::new(1, 2, 3);
        let vector4i = Vector4i::new(1, 2, 3, 4);
        // VectorNi to Vector2i
        assert_eq!(Vector2i::new(2, 1), swizzle!(vector2i => y, x));
        assert_eq!(Vector2i::new(2, 1), swizzle!(vector3i => y, x));
        assert_eq!(Vector2i::new(2, 1), swizzle!(vector4i => y, x));
        // VectorNi to Vector3i
        assert_eq!(Vector3i::new(2, 1, 1), swizzle!(vector2i => y, x, x));
        assert_eq!(Vector3i::new(2, 1, 3), swizzle!(vector3i => y, x, z));
        assert_eq!(Vector3i::new(2, 1, 3), swizzle!(vector4i => y, x, z));
        // VectorNi to Vector4i
        assert_eq!(Vector4i::new(2, 1, 1, 2), swizzle!(vector2i => y, x, x, y));
        assert_eq!(Vector4i::new(2, 1, 3, 2), swizzle!(vector3i => y, x, z, y));
        assert_eq!(Vector4i::new(2, 1, 3, 4), swizzle!(vector4i => y, x, z, w));
    }
}
