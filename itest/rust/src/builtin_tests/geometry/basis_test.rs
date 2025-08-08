/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::builtin::inner::InnerBasis;
use godot::builtin::math::assert_eq_approx;
use godot::builtin::{real, Basis, EulerOrder, RealConv, VariantOperator, Vector3, XformInv};

use crate::builtin_tests::common::assert_evaluate_approx_eq;
use crate::framework::itest;

const TEST_BASIS: Basis = Basis::from_rows(
    Vector3::new(0.942155, -0.270682, 0.197677),
    Vector3::new(0.294044, 0.950564, -0.099833),
    Vector3::new(-0.160881, 0.152184, 0.97517),
);

#[itest]
fn basis_multiply_same() {
    // operator: Basis * Identity
    assert_evaluate_approx_eq(
        TEST_BASIS,
        Basis::IDENTITY,
        VariantOperator::MULTIPLY,
        TEST_BASIS * Basis::IDENTITY,
    );

    // operator: Basis * rotated Basis
    let rotated_basis = Basis::from_axis_angle(Vector3::new(1.0, 2.0, 3.0).normalized(), 0.5);
    assert_evaluate_approx_eq(
        TEST_BASIS,
        rotated_basis,
        VariantOperator::MULTIPLY,
        TEST_BASIS * rotated_basis,
    );

    // orthonormalized
    let orthonormalized_basis = TEST_BASIS.orthonormalized();
    assert_evaluate_approx_eq(
        orthonormalized_basis,
        orthonormalized_basis.inverse(),
        VariantOperator::MULTIPLY,
        Basis::IDENTITY,
    );
    assert_evaluate_approx_eq(
        orthonormalized_basis,
        orthonormalized_basis.transposed(),
        VariantOperator::MULTIPLY,
        Basis::IDENTITY,
    );
}

#[itest]
fn basis_euler_angles_same() {
    let euler_order_to_test: Vec<EulerOrder> = vec![
        EulerOrder::XYZ,
        EulerOrder::XZY,
        EulerOrder::YZX,
        EulerOrder::YXZ,
        EulerOrder::ZXY,
        EulerOrder::ZYX,
    ];

    let vectors_to_test: Vec<Vector3> = vec![
        Vector3::new(0.0, 0.0, 0.0),
        Vector3::new(0.5, 0.5, 0.5),
        Vector3::new(-0.5, -0.5, -0.5),
        Vector3::new(40.0, 40.0, 40.0),
        Vector3::new(-40.0, -40.0, -40.0),
        Vector3::new(0.0, 0.0, -90.0),
        Vector3::new(0.0, -90.0, 0.0),
        Vector3::new(-90.0, 0.0, 0.0),
        Vector3::new(0.0, 0.0, 90.0),
        Vector3::new(0.0, 90.0, 0.0),
        Vector3::new(90.0, 0.0, 0.0),
        Vector3::new(0.0, 0.0, -30.0),
        Vector3::new(0.0, -30.0, 0.0),
        Vector3::new(-30.0, 0.0, 0.0),
        Vector3::new(0.0, 0.0, 30.0),
        Vector3::new(0.0, 30.0, 0.0),
        Vector3::new(30.0, 0.0, 0.0),
        Vector3::new(0.5, 50.0, 20.0),
        Vector3::new(-0.5, -50.0, -20.0),
        Vector3::new(0.5, 0.0, 90.0),
        Vector3::new(0.5, 0.0, -90.0),
        Vector3::new(360.0, 360.0, 360.0),
        Vector3::new(-360.0, -360.0, -360.0),
        Vector3::new(-90.0, 60.0, -90.0),
        Vector3::new(90.0, 60.0, -90.0),
        Vector3::new(90.0, -60.0, -90.0),
        Vector3::new(-90.0, -60.0, -90.0),
        Vector3::new(-90.0, 60.0, 90.0),
        Vector3::new(90.0, 60.0, 90.0),
        Vector3::new(90.0, -60.0, 90.0),
        Vector3::new(-90.0, -60.0, 90.0),
        Vector3::new(60.0, 90.0, -40.0),
        Vector3::new(60.0, -90.0, -40.0),
        Vector3::new(-60.0, -90.0, -40.0),
        Vector3::new(-60.0, 90.0, 40.0),
        Vector3::new(60.0, 90.0, 40.0),
        Vector3::new(60.0, -90.0, 40.0),
        Vector3::new(-60.0, -90.0, 40.0),
        Vector3::new(-90.0, 90.0, -90.0),
        Vector3::new(90.0, 90.0, -90.0),
        Vector3::new(90.0, -90.0, -90.0),
        Vector3::new(-90.0, -90.0, -90.0),
        Vector3::new(-90.0, 90.0, 90.0),
        Vector3::new(90.0, 90.0, 90.0),
        Vector3::new(90.0, -90.0, 90.0),
        Vector3::new(20.0, 150.0, 30.0),
        Vector3::new(20.0, -150.0, 30.0),
        Vector3::new(-120.0, -150.0, 30.0),
        Vector3::new(-120.0, -150.0, -130.0),
        Vector3::new(120.0, -150.0, -130.0),
        Vector3::new(120.0, 150.0, -130.0),
        Vector3::new(120.0, 150.0, 130.0),
    ];

    for order in euler_order_to_test.into_iter() {
        for vector in vectors_to_test.iter() {
            let vector = deg_to_rad(*vector);
            let rust_basis = Basis::from_euler(order, vector);
            let godot_basis = InnerBasis::from_euler(vector, order as i64);
            assert_eq_approx!(rust_basis, godot_basis);
        }
    }
}

#[itest]
fn basis_equiv() {
    let inner = InnerBasis::from_outer(&TEST_BASIS);
    let outer = TEST_BASIS;
    let vec = Vector3::new(1.0, 2.0, 3.0);

    #[rustfmt::skip]
    let mappings_basis = [
        ("inverse",         inner.inverse(),                      outer.inverse()                      ),
        ("transposed",      inner.transposed(),                   outer.transposed()                   ),
        ("orthonormalized", inner.orthonormalized(),              outer.orthonormalized()              ),
        ("rotated",         inner.rotated(vec.normalized(), 0.1), outer.rotated(vec.normalized(), 0.1) ),
        ("scaled",          inner.scaled(vec),                    outer.scaled(vec)                    ),
        ("slerp",           inner.slerp(Basis::IDENTITY, 0.5),    outer.slerp(&Basis::IDENTITY, 0.5)   ),
    ];
    for (name, inner, outer) in mappings_basis {
        assert_eq_approx!(inner, outer, "function: {name}\n");
    }

    #[rustfmt::skip]
    let mappings_float = [
        ("determinant", inner.determinant(), outer.determinant()),
        ("tdotx",       inner.tdotx(vec),    outer.tdotx(vec)   ),
        ("tdoty",       inner.tdoty(vec),    outer.tdoty(vec)   ),
        ("tdotz",       inner.tdotz(vec),    outer.tdotz(vec)   ),
    ];
    for (name, inner, outer) in mappings_float {
        assert_eq_approx!(real::from_f64(inner), outer, "function: {name}\n");
    }

    assert_eq_approx!(
        inner.get_scale(),
        outer.get_scale(),
        "function: get_scale\n"
    );
    assert_eq_approx!(
        inner.get_euler(EulerOrder::XYZ as i64),
        outer.get_euler_with(EulerOrder::XYZ),
        "function: get_euler\n"
    );
    assert_eq_approx!(
        inner.get_rotation_quaternion(),
        outer.get_quaternion(),
        "function: get_rotation_quaternion\n"
    )
}

#[itest]
fn basis_xform_equiv() {
    let orthonormalized_basis = TEST_BASIS.orthonormalized();
    let vec = Vector3::new(1.0, 2.0, 3.0);

    // operator: Basis * Vector3
    assert_evaluate_approx_eq(
        orthonormalized_basis,
        vec,
        VariantOperator::MULTIPLY,
        orthonormalized_basis * vec,
    );
}

#[itest]
fn basis_xform_inv_equiv() {
    let orthonormalized_basis = TEST_BASIS.orthonormalized();
    let vec = Vector3::new(1.0, 2.0, 3.0);

    // operator: Vector3 * Basis
    assert_evaluate_approx_eq(
        vec,
        orthonormalized_basis,
        VariantOperator::MULTIPLY,
        orthonormalized_basis.xform_inv(vec),
    );
}

fn deg_to_rad(rotation: Vector3) -> Vector3 {
    Vector3::new(
        rotation.x.to_radians(),
        rotation.y.to_radians(),
        rotation.z.to_radians(),
    )
}
