/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::fmt::Debug;

use godot::builtin::inner::InnerPlane;
use godot::builtin::math::{assert_eq_approx, ApproxEq};
use godot::builtin::{real, Plane, RealConv, Vector3};
use godot::meta::ToGodot;

use crate::framework::itest;

fn check_mapping_eq<T>(context: &str, outer: T, inner: T)
where
    T: PartialEq + Debug,
{
    assert_eq!(
        outer, inner,
        "{context}: outer != inner ({outer:?} != {inner:?})"
    );
}

fn check_mapping_eq_approx_plane(context: &str, outer: Plane, inner: Plane) {
    assert_eq_approx!(outer, inner, "{context}");
}

#[itest]
fn plane_normalized() {
    let a = Plane::new(Vector3::new(9.5, 3.3, 2.2).normalized(), -0.1);
    let inner_a = InnerPlane::from_outer(&a);
    check_mapping_eq_approx_plane("normalized", a.normalized(), inner_a.normalized());

    let a = Plane {
        normal: Vector3::new(4.2, 2.9, 1.5),
        d: 2.4,
    };
    let inner_a = InnerPlane::from_outer(&a);
    check_mapping_eq_approx_plane("normalized", a.normalized(), inner_a.normalized());
}

#[itest]
fn plane_center() {
    let a = Plane::new(Vector3::new(0.5, 2.0, 2.5).normalized(), 1.0);
    let inner_a = InnerPlane::from_outer(&a);
    check_mapping_eq("center", a.center(), inner_a.get_center());
}

#[itest]
fn plane_is_finite() {
    let a = Plane::new(Vector3::new(9.4, -1.2, 3.0).normalized(), 1.5);
    let inner_a = InnerPlane::from_outer(&a);
    check_mapping_eq("is_finite", a.is_finite(), inner_a.is_finite());

    let a = Plane {
        normal: Vector3::new(real::INFINITY, 2.9, 1.5),
        d: 2.4,
    };
    let inner_a = InnerPlane::from_outer(&a);
    check_mapping_eq("is_finite", a.is_finite(), inner_a.is_finite());
}

#[itest]
fn plane_distance_to() {
    let a = Plane::new(Vector3::BACK, 0.2);
    let inner_a = InnerPlane::from_outer(&a);
    let b = Vector3::new(7.0, 9.0, 2.0);
    check_mapping_eq(
        "distance_to",
        a.distance_to(b).as_f64(),
        inner_a.distance_to(b),
    );

    let b = Vector3::new(-7.0, -9.0, -2.0);
    check_mapping_eq(
        "distance_to",
        a.distance_to(b).as_f64(),
        inner_a.distance_to(b),
    );
}

#[itest]
fn plane_is_point_over() {
    let a = Plane::new(Vector3::BACK, 1.1);
    let inner_a = InnerPlane::from_outer(&a);
    let b = Vector3::new(6.7, 8.4, 8.3);
    check_mapping_eq(
        "is_point_over",
        a.is_point_over(b),
        inner_a.is_point_over(b),
    );

    let b = Vector3::new(-0.5, -1.2, -7.1);
    check_mapping_eq(
        "is_point_over",
        a.is_point_over(b),
        inner_a.is_point_over(b),
    );
}

#[itest]
fn plane_project() {
    let a = Plane::new(Vector3::new(8.6, -1.1, -10.1).normalized(), 3.3);
    let inner_a = InnerPlane::from_outer(&a);
    let b = Vector3::new(7.2, -3.4, 9.9);
    check_mapping_eq("project", a.project(b), inner_a.project(b));
}

#[itest]
fn plane_intersect_segment() {
    let a = Plane::new(Vector3::BACK, -0.2);
    let inner_a = InnerPlane::from_outer(&a);
    let b = Vector3::new(7.2, 10.4, 9.9);
    let c = Vector3::new(-9.4, -3.4, -3.1);
    check_mapping_eq(
        "intersect_segment",
        a.intersect_segment(b, c)
            .as_ref()
            .map(ToGodot::to_variant)
            .unwrap_or_default(),
        inner_a.intersects_segment(b, c),
    );

    let a = Plane::new(Vector3::BACK, 0.0);
    let inner_a = InnerPlane::from_outer(&a);
    let b = Vector3::BACK;
    let c = Vector3::new(0.0, 0.0, 0.5);
    check_mapping_eq(
        "intersect_segment",
        a.intersect_segment(b, c)
            .as_ref()
            .map(ToGodot::to_variant)
            .unwrap_or_default(),
        inner_a.intersects_segment(b, c),
    );
}

#[itest]
fn plane_intersect_ray() {
    let a = Plane::new(Vector3::BACK, 0.0);
    let inner_a = InnerPlane::from_outer(&a);
    let b = Vector3::new(0.1, 9.9, 5.7);
    let c = Vector3::new(3.5, 3.2, 0.3);
    check_mapping_eq(
        "intersect_ray",
        a.intersect_ray(b, c)
            .as_ref()
            .map(ToGodot::to_variant)
            .unwrap_or_default(),
        inner_a.intersects_ray(b, c),
    );

    let b = Vector3::BACK;
    let c = Vector3::new(1.0, 0.0, 1.0);
    check_mapping_eq(
        "intersect_ray",
        a.intersect_ray(b, c)
            .as_ref()
            .map(ToGodot::to_variant)
            .unwrap_or_default(),
        inner_a.intersects_ray(b, c),
    );
}

#[itest]
fn plane_contains_point() {
    let a = Plane::new(Vector3::new(0.9, 6.6, 0.1).normalized(), 0.0001);
    let inner_a = InnerPlane::from_outer(&a);
    let b = Vector3::ZERO;
    let c: real = 0.01;
    check_mapping_eq(
        "contains_point",
        a.contains_point(b, Some(c)),
        inner_a.has_point(b, c.as_f64()),
    );

    let a = Plane::new(Vector3::new(0.9, 6.6, 0.1).normalized(), 0.1);
    let inner_a = InnerPlane::from_outer(&a);
    check_mapping_eq(
        "contains_point",
        a.contains_point(b, Some(c)),
        inner_a.has_point(b, c.as_f64()),
    );
}

#[itest]
fn plane_is_equal_approx() {
    let a = Plane::new(Vector3::new(1.5, 6.3, 2.2).normalized(), 5.2);
    let inner_a = InnerPlane::from_outer(&a);
    let b = Plane::new(Vector3::new(1.5, 6.3, 2.2).normalized(), 5.2000001);
    check_mapping_eq(
        "is_equal_approx",
        a.approx_eq(&b),
        inner_a.is_equal_approx(b),
    );

    let a = Plane::new(Vector3::new(-1.9, 9.0, 2.7).normalized(), 5.4);
    let inner_a = InnerPlane::from_outer(&a);
    let b = Plane::new(Vector3::new(0.0, 6.2, 2.5).normalized(), 0.4);
    check_mapping_eq(
        "is_equal_approx",
        a.approx_eq(&b),
        inner_a.is_equal_approx(b),
    );
}

#[itest]
fn plane_intersect_3() {
    let a = Plane::new(Vector3::new(1.0, 2.0, 0.0).normalized(), 0.0);
    let inner_a = InnerPlane::from_outer(&a);
    let b = Plane::new(Vector3::new(3.5, 6.0, -3.0).normalized(), 0.0);
    let c = Plane::new(Vector3::new(-1.0, 6.0, 0.5).normalized(), 0.0);
    check_mapping_eq(
        "intersect_3",
        a.intersect_3(b, c)
            .as_ref()
            .map(ToGodot::to_variant)
            .unwrap_or_default(),
        inner_a.intersect_3(b, c),
    );

    let a = Plane::new(Vector3::new(1.5, 6.3, 2.2).normalized(), 5.2);
    let inner_a = InnerPlane::from_outer(&a);
    let b = Plane::new(Vector3::new(1.5, 6.3, 2.2).normalized(), 3.2);
    let c = Plane::new(Vector3::new(1.5, 6.3, 2.2).normalized(), 9.5);
    check_mapping_eq(
        "intersect_3",
        a.intersect_3(b, c)
            .as_ref()
            .map(ToGodot::to_variant)
            .unwrap_or_default(),
        inner_a.intersect_3(b, c),
    );
}
