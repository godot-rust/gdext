/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::builtin::inner::InnerTransform3D;
use godot::builtin::{Aabb, Basis, Plane, Transform3D, VariantOperator, Vector3, XformInv};
use godot::private::class_macros::assert_eq_approx;

use crate::builtin_tests::common::assert_evaluate_approx_eq;
use crate::framework::itest;

const TEST_TRANSFORM: Transform3D = Transform3D::new(
    Basis::from_cols(
        Vector3::new(1.0, 2.0, 3.0),
        Vector3::new(4.0, 5.0, 6.0),
        Vector3::new(7.0, 8.0, -9.0),
    ),
    Vector3::new(10.0, 11.0, 12.0),
);

const TEST_TRANSFORM_ORTHONORMAL: Transform3D =
    Transform3D::new(Basis::IDENTITY, Vector3::new(10.0, 11.0, 12.0));

#[itest]
fn transform3d_equiv() {
    let inner = InnerTransform3D::from_outer(&TEST_TRANSFORM);
    let outer = TEST_TRANSFORM;
    let vec = Vector3::new(1.0, 2.0, 3.0);

    #[rustfmt::skip]
    let mappings_transform = [
        ("affine_inverse",   inner.affine_inverse(),                             outer.affine_inverse()                             ),
        ("orthonormalized",  inner.orthonormalized(),                            outer.orthonormalized()                            ),
        ("rotated",          inner.rotated(vec.normalized(), 1.0),               outer.rotated(vec.normalized(), 1.0)               ),
        ("rotated_local",    inner.rotated_local(vec.normalized(), 1.0),         outer.rotated_local(vec.normalized(), 1.0)         ),
        ("scaled",           inner.scaled(vec),                                  outer.scaled(vec)                                  ),
        ("scaled_local",     inner.scaled_local(vec),                            outer.scaled_local(vec)                            ),
        ("translated",       inner.translated(vec),                              outer.translated(vec)                              ),
        ("translated_local", inner.translated_local(vec),                        outer.translated_local(vec)                        ),
        ("interpolate_with", inner.interpolate_with(Transform3D::IDENTITY, 0.5), outer.interpolate_with(&Transform3D::IDENTITY, 0.5))
    ];
    for (name, inner, outer) in mappings_transform {
        assert_eq_approx!(inner, outer, "function: {name}\n");
    }
}

#[itest]
fn transform3d_xform_equiv() {
    let vec = Vector3::new(1.0, 2.0, 3.0);

    // operator: Transform3D * Vector3
    assert_evaluate_approx_eq(
        TEST_TRANSFORM,
        vec,
        VariantOperator::MULTIPLY,
        TEST_TRANSFORM * vec,
    );

    let aabb = Aabb::new(Vector3::new(1.0, 2.0, 3.0), Vector3::new(4.0, 5.0, 6.0));

    // operator: Transform3D * Aabb
    assert_evaluate_approx_eq(
        TEST_TRANSFORM,
        aabb,
        VariantOperator::MULTIPLY,
        TEST_TRANSFORM * aabb,
    );

    let plane = Plane::new(Vector3::new(1.0, 2.0, 3.0).normalized(), 5.0);

    // operator: Transform3D * Plane
    assert_evaluate_approx_eq(
        TEST_TRANSFORM,
        plane,
        VariantOperator::MULTIPLY,
        TEST_TRANSFORM * plane,
    );
}

#[itest]
fn transform3d_xform_inv_equiv() {
    let vec = Vector3::new(1.0, 2.0, 3.0);

    // operator: Vector3 * Transform3D
    assert_evaluate_approx_eq(
        vec,
        TEST_TRANSFORM_ORTHONORMAL,
        VariantOperator::MULTIPLY,
        TEST_TRANSFORM_ORTHONORMAL.xform_inv(vec),
    );

    let aabb = Aabb::new(Vector3::new(1.0, 2.0, 3.0), Vector3::new(4.0, 5.0, 6.0));

    // operator: Aabb * Transform3D  (1)
    assert_evaluate_approx_eq(
        aabb,
        TEST_TRANSFORM_ORTHONORMAL,
        VariantOperator::MULTIPLY,
        TEST_TRANSFORM_ORTHONORMAL.xform_inv(aabb),
    );

    let transform_rotated =
        TEST_TRANSFORM_ORTHONORMAL.rotated(Vector3::new(0.2, 0.4, 1.0).normalized(), 0.8);

    // operator: Aabb * Transform3D (2)
    assert_evaluate_approx_eq(
        aabb,
        transform_rotated,
        VariantOperator::MULTIPLY,
        transform_rotated.xform_inv(aabb),
    );

    let plane = Plane::new(Vector3::new(1.0, 2.0, 3.0).normalized(), 5.0);

    // operator: Plane * Transform3D
    assert_evaluate_approx_eq(
        plane,
        TEST_TRANSFORM_ORTHONORMAL,
        VariantOperator::MULTIPLY,
        TEST_TRANSFORM_ORTHONORMAL.xform_inv(plane),
    );
}
