/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::builtin::inner::InnerTransform2D;
use godot::builtin::{real, RealConv, Rect2, Transform2D, VariantOperator, Vector2, XformInv};
use godot::private::class_macros::assert_eq_approx;

use crate::builtin_tests::common::assert_evaluate_approx_eq;
use crate::framework::itest;

const TEST_TRANSFORM: Transform2D = Transform2D::from_cols(
    Vector2::new(1.0, 2.0),
    Vector2::new(3.0, 4.0),
    Vector2::new(5.0, 6.0),
);

const TEST_TRANSFORM_ORTHONORMAL: Transform2D = Transform2D::from_cols(
    Vector2::new(1.0, 0.0),
    Vector2::new(0.0, 1.0),
    Vector2::new(5.0, 6.0),
);

#[itest]
fn transform2d_equiv() {
    let inner = InnerTransform2D::from_outer(&TEST_TRANSFORM);
    let outer = TEST_TRANSFORM;
    let vec = Vector2::new(1.0, 2.0);

    #[rustfmt::skip]
        let mappings_transform = [
        ("affine_inverse",   inner.affine_inverse(),                             outer.affine_inverse()                             ),
        ("orthonormalized",  inner.orthonormalized(),                            outer.orthonormalized()                            ),
        ("rotated",          inner.rotated(1.0),                                 outer.rotated(1.0)                                 ),
        ("rotated_local",    inner.rotated_local(1.0),                           outer.rotated_local(1.0)                           ),
        ("scaled",           inner.scaled(vec),                                  outer.scaled(vec)                                  ),
        ("scaled_local",     inner.scaled_local(vec),                            outer.scaled_local(vec)                            ),
        ("translated",       inner.translated(vec),                              outer.translated(vec)                              ),
        ("translated_local", inner.translated_local(vec),                        outer.translated_local(vec)                        ),
        ("interpolate_with", inner.interpolate_with(Transform2D::IDENTITY, 0.5), outer.interpolate_with(&Transform2D::IDENTITY, 0.5))
    ];
    for (name, inner, outer) in mappings_transform {
        assert_eq_approx!(inner, outer, "function: {name}\n");
    }

    assert_eq_approx!(
        real::from_f64(inner.get_rotation()),
        outer.rotation(),
        "function: get_rotation\n"
    );
    assert_eq_approx!(
        real::from_f64(inner.get_rotation()),
        outer.rotation(),
        "function: get_rotation\n"
    );
    assert_eq_approx!(
        real::from_f64(inner.get_skew()),
        outer.skew(),
        "function: get_scale\n"
    );
}

#[itest]
fn transform2d_determinant() {
    let inner = InnerTransform2D::from_outer(&TEST_TRANSFORM);
    let outer = TEST_TRANSFORM;

    assert_eq_approx!(
        real::from_f64(inner.determinant()),
        outer.determinant(),
        "function: determinant\n"
    );
}

#[itest]
fn transform2d_xform_equiv() {
    let vec = Vector2::new(1.0, 2.0);

    // operator: Transform2D * Vector2
    assert_evaluate_approx_eq(
        TEST_TRANSFORM,
        vec,
        VariantOperator::MULTIPLY,
        TEST_TRANSFORM * vec,
    );

    let rect_2 = Rect2::new(Vector2::new(1.0, 2.0), Vector2::new(3.0, 4.0));

    // operator: Transform2D * Rect2 (1)
    assert_evaluate_approx_eq(
        TEST_TRANSFORM,
        rect_2,
        VariantOperator::MULTIPLY,
        TEST_TRANSFORM * rect_2,
    );

    // "operator: Transform2D * Rect2 (2)"
    let transform_rotated = TEST_TRANSFORM_ORTHONORMAL.rotated(0.8);
    assert_evaluate_approx_eq(
        transform_rotated,
        rect_2,
        VariantOperator::MULTIPLY,
        transform_rotated * rect_2,
    );
}

#[itest]
fn transform2d_xform_inv_equiv() {
    let vec = Vector2::new(1.0, 2.0);

    // operator: Vector2 * Transform2D
    assert_evaluate_approx_eq(
        vec,
        TEST_TRANSFORM_ORTHONORMAL,
        VariantOperator::MULTIPLY,
        TEST_TRANSFORM_ORTHONORMAL.xform_inv(vec),
    );

    let rect_2 = Rect2::new(Vector2::new(1.0, 2.0), Vector2::new(3.0, 4.0));

    // operator: Rect2 * Transform2D (1)
    assert_evaluate_approx_eq(
        rect_2,
        TEST_TRANSFORM_ORTHONORMAL,
        VariantOperator::MULTIPLY,
        TEST_TRANSFORM_ORTHONORMAL.xform_inv(rect_2),
    );

    // operator: Rect2 * Transform2D (2)
    let transform_rotated = TEST_TRANSFORM_ORTHONORMAL.rotated(0.8);
    assert_evaluate_approx_eq(
        rect_2,
        transform_rotated,
        VariantOperator::MULTIPLY,
        transform_rotated.xform_inv(rect_2),
    );
}
