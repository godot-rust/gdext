/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

#![allow(clippy::type_complexity, clippy::excessive_precision)]

use crate::itest;
use godot::prelude::{inner::InnerProjection, *};
use godot::private::class_macros::assert_eq_approx;

fn matrix_eq_approx(a: Projection, b: Projection) -> bool {
    for i in 0..4 {
        let v1 = a.cols[i];
        let v2 = b.cols[i];
        if !is_equal_approx(v1.x, v2.x)
            && !is_equal_approx(v1.y, v2.y)
            && !is_equal_approx(v1.z, v2.z)
            && !is_equal_approx(v1.w, v2.w)
        {
            return false;
        }
    }
    true
}

#[itest]
fn test_create_orthogonal() {
    const TEST_DATA: [[real; 6]; 6] = [
        [-1.0, 1.0, -1.0, 1.0, -1.0, 1.0],
        [0.0, 1.0, 0.0, 1.0, 0.0, 1.0],
        [-1.0, 1.0, -1.0, 1.0, 0.0, 1.0],
        [-10.0, 10.0, -10.0, 10.0, 0.0, 100.0],
        [-1.0, 1.0, -1.0, 1.0, 1.0, -1.0],
        [10.0, -10.0, 10.0, -10.0, -10.0, 10.0],
    ];

    for [left, right, bottom, top, near, far] in TEST_DATA {
        let rust_proj = Projection::create_orthogonal(left, right, bottom, top, near, far);
        let godot_proj = InnerProjection::create_orthogonal(
            left as _,
            right as _,
            bottom as _,
            top as _,
            near as _,
            far as _,
        );

        assert_eq_approx!(
            rust_proj,
            godot_proj,
            matrix_eq_approx,
            "left={left} right={right} bottom={bottom} top={top} near={near} far={far}"
        );
    }
}

#[itest]
fn test_create_orthogonal_aspect() {
    const TEST_DATA: [(real, real, real, real, bool); 6] = [
        (2.0, 1.0, 0.0, 1.0, false),
        (2.0, 1.0, 0.0, 1.0, true),
        (1.0, 2.0, 0.0, 100.0, false),
        (1.0, 2.0, 0.0, 100.0, true),
        (64.0, 9.0 / 16.0, 0.0, 100.0, false),
        (64.0, 9.0 / 16.0, 0.0, 100.0, true),
    ];

    for (size, aspect, near, far, flip_fov) in TEST_DATA {
        let rust_proj = Projection::create_orthogonal_aspect(size, aspect, near, far, flip_fov);
        let godot_proj = InnerProjection::create_orthogonal_aspect(
            size as _,
            aspect as _,
            near as _,
            far as _,
            flip_fov,
        );

        assert_eq_approx!(
            rust_proj,
            godot_proj,
            matrix_eq_approx,
            "size={size} aspect={aspect} near={near} far={far} flip_fov={flip_fov}"
        );
    }
}

#[itest]
fn test_create_perspective() {
    const TEST_DATA: [(real, real, real, real, bool); 5] = [
        (90.0, 1.0, 1.0, 2.0, false),
        (90.0, 1.0, 1.0, 2.0, true),
        (45.0, 1.0, 0.05, 100.0, false),
        (90.0, 9.0 / 16.0, 1.0, 2.0, false),
        (90.0, 9.0 / 16.0, 1.0, 2.0, true),
    ];

    for (fov_y, aspect, near, far, flip_fov) in TEST_DATA {
        let rust_proj = Projection::create_perspective(fov_y, aspect, near, far, flip_fov);
        let godot_proj = InnerProjection::create_perspective(
            fov_y as _,
            aspect as _,
            near as _,
            far as _,
            flip_fov,
        );

        assert_eq_approx!(
            rust_proj,
            godot_proj,
            matrix_eq_approx,
            "fov_y={fov_y} aspect={aspect} near={near} far={far} flip_fov={flip_fov}"
        );
    }
}

#[itest]
fn test_create_frustum() {
    const TEST_DATA: [[real; 6]; 3] = [
        [-1.0, 1.0, -1.0, 1.0, 1.0, 2.0],
        [0.0, 1.0, 0.0, 1.0, 1.0, 2.0],
        [-0.1, 0.1, -0.025, 0.025, 0.05, 100.0],
    ];

    for [left, right, bottom, top, near, far] in TEST_DATA {
        let rust_proj = Projection::create_frustum(left, right, bottom, top, near, far);
        let godot_proj = InnerProjection::create_frustum(
            left as _,
            right as _,
            bottom as _,
            top as _,
            near as _,
            far as _,
        );

        assert_eq_approx!(
            rust_proj,
            godot_proj,
            matrix_eq_approx,
            "left={left} right={right} bottom={bottom} top={top} near={near} far={far}"
        );
    }
}

#[itest]
fn test_create_frustum_aspect() {
    const TEST_DATA: [(real, real, Vector2, real, real, bool); 4] = [
        (2.0, 1.0, Vector2::ZERO, 1.0, 2.0, false),
        (2.0, 1.0, Vector2::ZERO, 1.0, 2.0, true),
        (1.0, 1.0, Vector2::new(0.5, 0.5), 1.0, 2.0, false),
        (0.05, 4.0, Vector2::ZERO, 0.05, 100.0, false),
    ];

    for (size, aspect, offset, near, far, flip_fov) in TEST_DATA {
        let rust_proj =
            Projection::create_frustum_aspect(size, aspect, offset, near, far, flip_fov);
        let godot_proj = InnerProjection::create_frustum_aspect(
            size as _,
            aspect as _,
            offset,
            near as _,
            far as _,
            flip_fov,
        );

        assert_eq_approx!(
            rust_proj,
            godot_proj,
            matrix_eq_approx,
            "size={size} aspect={aspect} offset=({0} {1}) near={near} far={far} flip_fov={flip_fov}",
            offset.x,
            offset.y,
        );
    }
}

#[itest]
fn test_projection_combined() {
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
                            let rust_proj =
                                Projection::create_orthogonal(left, right, bottom, top, near, far);
                            let godot_proj = InnerProjection::create_orthogonal(
                                left as _,
                                right as _,
                                bottom as _,
                                top as _,
                                near as _,
                                far as _,
                            );

                            assert_eq_approx!(
                                rust_proj,
                                godot_proj,
                                matrix_eq_approx,
                                "left={left} right={right} bottom={bottom} top={top} near={near} far={far}"
                            );

                            assert!(
                                InnerProjection::from_outer(&rust_proj).is_orthogonal(),
                                "Projection should be orthogonal (left={left} right={right} bottom={bottom} top={top} near={near} far={far})",
                            );
                        }
                    }
                }
            }
        }
    }

    // Perspective
    for fov_y in (0..18).map(|v| (v as real) * 10.0) {
        for aspect_x in 1..=10 {
            for aspect_y in 1..=10 {
                let aspect = (aspect_x as real) / (aspect_y as real);
                for near_i in 1..10 {
                    let near = near_i as real;
                    for far in (near_i + 1..=20).map(|v| v as real) {
                        let rust_proj =
                            Projection::create_perspective(fov_y, aspect, near, far, false);
                        let godot_proj = InnerProjection::create_perspective(
                            fov_y as _,
                            aspect as _,
                            near as _,
                            far as _,
                            false,
                        );

                        assert_eq_approx!(
                            rust_proj,
                            godot_proj,
                            matrix_eq_approx,
                            "fov_y={fov_y} aspect={aspect} near={near} far={far}"
                        );

                        assert!(
                            !InnerProjection::from_outer(&rust_proj).is_orthogonal(),
                            "Projection should be perspective (fov_y={fov_y} aspect={aspect} near={near} far={far})",
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
                            let rust_proj =
                                Projection::create_frustum(left, right, bottom, top, near, far);
                            let godot_proj = InnerProjection::create_frustum(
                                left as _,
                                right as _,
                                bottom as _,
                                top as _,
                                near as _,
                                far as _,
                            );

                            assert_eq_approx!(
                                rust_proj,
                                godot_proj,
                                matrix_eq_approx,
                                "left={left} right={right} bottom={bottom} top={top} near={near} far={far}"
                            );

                            assert!(
                                !InnerProjection::from_outer(&rust_proj).is_orthogonal(),
                                "Projection should be perspective (left={left} right={right} bottom={bottom} top={top} near={near} far={far})",
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
                        let rust_proj_frustum = Projection::create_frustum_aspect(
                            size,
                            aspect,
                            Vector2::ZERO,
                            near,
                            far,
                            false,
                        );
                        let godot_proj_frustum = InnerProjection::create_frustum_aspect(
                            size as _,
                            aspect as _,
                            Vector2::ZERO,
                            near as _,
                            far as _,
                            false,
                        );

                        assert_eq_approx!(
                            rust_proj_frustum,
                            godot_proj_frustum,
                            matrix_eq_approx,
                            "size={size} aspect={aspect} near={near} far={far}"
                        );

                        let rust_proj_ortho =
                            Projection::create_orthogonal_aspect(size, aspect, near, far, false);
                        let godot_proj_ortho = InnerProjection::create_orthogonal_aspect(
                            size as _,
                            aspect as _,
                            near as _,
                            far as _,
                            false,
                        );

                        assert_eq_approx!(
                            rust_proj_ortho,
                            godot_proj_ortho,
                            matrix_eq_approx,
                            "size={size} aspect={aspect} near={near} far={far}"
                        );

                        assert!(
                            InnerProjection::from_outer(&rust_proj_ortho).is_orthogonal(),
                            "Projection should be orthogonal (size={size} aspect={aspect} near={near} far={far})",
                        );
                        assert!(
                            !InnerProjection::from_outer(&rust_proj_frustum).is_orthogonal(),
                            "Projection should be perspective (size={size} aspect={aspect} near={near} far={far})",
                        );
                    }
                }
            }
        }
    }
}
