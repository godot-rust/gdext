/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
use crate::itest;

use godot::prelude::{inner::InnerTransform3D, *};
use godot::private::class_macros::assert_eq_approx;

const TEST_TRANSFORM: Transform3D = Transform3D::new(
    Basis::from_cols(
        Vector3::new(1.0, 2.0, 3.0),
        Vector3::new(4.0, 5.0, 6.0),
        Vector3::new(7.0, 8.0, -9.0),
    ),
    Vector3::new(10.0, 11.0, 12.0),
);

#[itest]
fn transform3d_equiv() {
    let inner = InnerTransform3D::from_outer(&TEST_TRANSFORM);
    let outer = TEST_TRANSFORM;
    let vec = Vector3::new(1.0, 2.0, 3.0);

    #[rustfmt::skip]
    let mappings_transform = [
        ("affine_inverse",   inner.affine_inverse(),                             outer.affine_inverse()                            ),
        ("orthonormalized",  inner.orthonormalized(),                            outer.orthonormalized()                           ),
        ("rotated",          inner.rotated(vec.normalized(), 1.0),               outer.rotated(vec.normalized(), 1.0)              ),
        ("rotated_local",    inner.rotated_local(vec.normalized(), 1.0),         outer.rotated_local(vec.normalized(), 1.0)        ),
        ("scaled",           inner.scaled(vec),                                  outer.scaled(vec)                                 ),
        ("scaled_local",     inner.scaled_local(vec),                            outer.scaled_local(vec)                           ),
        ("translated",       inner.translated(vec),                              outer.translated(vec)                             ),
        ("translated_local", inner.translated_local(vec),                        outer.translated_local(vec)                       ),
        ("interpolate_with", inner.interpolate_with(Transform3D::IDENTITY, 0.5), outer.interpolate_with(Transform3D::IDENTITY, 0.5))
    ];
    for (name, inner, outer) in mappings_transform {
        assert_eq_approx!(
            &inner,
            &outer,
            Transform3D::is_equal_approx,
            "function: {name}\n"
        );
    }
}

#[itest]
fn transform3d_xform_equiv() {
    let vec = Vector3::new(1.0, 2.0, 3.0);

    assert_eq_approx!(
        TEST_TRANSFORM * vec,
        TEST_TRANSFORM
            .to_variant()
            .evaluate(&vec.to_variant(), VariantOperator::Multiply)
            .unwrap()
            .to::<Vector3>(),
        Vector3::is_equal_approx,
        "operator: Transform3D * Vector3"
    );

    let aabb = Aabb::new(Vector3::new(1.0, 2.0, 3.0), Vector3::new(4.0, 5.0, 6.0));

    assert_eq_approx!(
        TEST_TRANSFORM * aabb,
        TEST_TRANSFORM
            .to_variant()
            .evaluate(&aabb.to_variant(), VariantOperator::Multiply)
            .unwrap()
            .to::<Aabb>(),
        |a, b| Aabb::is_equal_approx(&a, &b),
        "operator: Transform3D * Aabb"
    );

    let plane = Plane::new(Vector3::new(1.0, 2.0, 3.0).normalized(), 5.0);

    assert_eq_approx!(
        TEST_TRANSFORM * plane,
        TEST_TRANSFORM
            .to_variant()
            .evaluate(&plane.to_variant(), VariantOperator::Multiply)
            .unwrap()
            .to::<Plane>(),
        |a, b| Plane::is_equal_approx(&a, &b),
        "operator: Transform3D * Plane"
    );
}
