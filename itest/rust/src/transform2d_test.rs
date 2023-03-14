/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
use crate::itest;

use godot::prelude::{inner::InnerTransform2D, *};
use godot::private::class_macros::assert_eq_approx;

const TEST_TRANSFORM: Transform2D = Transform2D::from_cols(
    Vector2::new(1.0, 2.0),
    Vector2::new(3.0, 4.0),
    Vector2::new(5.0, 6.0),
);

#[itest]
fn transform2d_equiv() {
    let inner = InnerTransform2D::from_outer(&TEST_TRANSFORM);
    let outer = TEST_TRANSFORM;
    let vec = Vector2::new(1.0, 2.0);

    #[rustfmt::skip]
    let mappings_transform = [
        ("affine_inverse",   inner.affine_inverse(),                             outer.affine_inverse()                            ),
        ("orthonormalized",  inner.orthonormalized(),                            outer.orthonormalized()                           ),
        ("rotated",          inner.rotated(1.0),                                 outer.rotated(1.0)                                ),
        ("rotated_local",    inner.rotated_local(1.0),                           outer.rotated_local(1.0)                          ),
        ("scaled",           inner.scaled(vec),                                  outer.scaled(vec)                                 ),
        ("scaled_local",     inner.scaled_local(vec),                            outer.scaled_local(vec)                           ),
        ("translated",       inner.translated(vec),                              outer.translated(vec)                             ),
        ("translated_local", inner.translated_local(vec),                        outer.translated_local(vec)                       ),
        ("interpolate_with", inner.interpolate_with(Transform2D::IDENTITY, 0.5), outer.interpolate_with(Transform2D::IDENTITY, 0.5))
    ];
    for (name, inner, outer) in mappings_transform {
        assert_eq_approx!(
            &inner,
            &outer,
            Transform2D::is_equal_approx,
            "function: {name}\n"
        );
    }

    assert_eq_approx!(
        inner.get_rotation(),
        outer.rotation(),
        |a, b| is_equal_approx(real::from_f64(a), b),
        "function: get_rotation\n"
    );
    assert_eq_approx!(
        inner.get_rotation(),
        outer.rotation(),
        |a, b| is_equal_approx(real::from_f64(a), b),
        "function: get_rotation\n"
    );
    assert_eq_approx!(
        inner.get_skew(),
        outer.skew(),
        |a, b| is_equal_approx(real::from_f64(a), b),
        "function: get_scale\n"
    );
}

#[itest]
fn transform2d_xform_equiv() {
    let vec = Vector2::new(1.0, 2.0);

    assert_eq_approx!(
        TEST_TRANSFORM * vec,
        TEST_TRANSFORM
            .to_variant()
            .evaluate(&vec.to_variant(), VariantOperator::Multiply)
            .unwrap()
            .to::<Vector2>(),
        Vector2::is_equal_approx,
        "operator: Transform2D * Vector2"
    );

    let rect_2 = Rect2::new(Vector2::new(1.0, 2.0), Vector2::new(3.0, 4.0));

    assert_eq_approx!(
        TEST_TRANSFORM * rect_2,
        TEST_TRANSFORM
            .to_variant()
            .evaluate(&rect_2.to_variant(), VariantOperator::Multiply)
            .unwrap()
            .to::<Rect2>(),
        |a, b| Rect2::is_equal_approx(&a, &b),
        "operator: Transform2D * Rect2 (1)"
    );

    assert_eq_approx!(
        TEST_TRANSFORM.rotated(0.8) * rect_2,
        TEST_TRANSFORM
            .rotated(0.8)
            .to_variant()
            .evaluate(&rect_2.to_variant(), VariantOperator::Multiply)
            .unwrap()
            .to::<Rect2>(),
        |a, b| Rect2::is_equal_approx(&a, &b),
        "operator: Transform2D * Rect2 (2)"
    );
}
