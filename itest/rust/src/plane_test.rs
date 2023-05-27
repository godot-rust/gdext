/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::fmt::Debug;

use crate::itest;
use godot::{
    prelude::{inner::InnerPlane, real, Plane, RealConv, ToVariant, Vector3},
    private::class_macros::assert_eq_approx,
};

#[itest]
fn plane_equiv() {
    let test_planes = [
        Plane::new(Vector3::UP.normalized(), 0.0),
        Plane::new(Vector3::BACK.normalized(), 0.0),
        Plane::new(Vector3::new(1.5, 3.0, 0.0).normalized(), 2.0),
        Plane::new(Vector3::new(0.5, 2.0, 2.5).normalized(), 1.0),
        Plane::new(Vector3::new(-3.0, 5.0, 0.1).normalized(), -0.2),
        Plane::new(Vector3::new(1.82, 5.32, -6.1).normalized(), 6.0),
        Plane::new(Vector3::new(1.82, 5.32, -6.000001).normalized(), 6.0),
    ];
    let unnormalized_planes = [
        Plane {
            normal: Vector3::new(4.2, 2.9, 1.5),
            d: 2.4,
        },
        Plane {
            normal: Vector3::new(-7.4, 10.5, -1.5),
            d: 6.0,
        },
        Plane {
            normal: Vector3::new(-3.1, 0.0, 1.4),
            d: 6.1,
        },
        Plane {
            normal: Vector3::new(-5.0, 12.0, 9.1),
            d: -2.4,
        },
        Plane {
            normal: Vector3::new(0.0, 0.0, 0.0),
            d: 6.0,
        },
    ];
    let test_vectors = [
        Vector3::ZERO,
        Vector3::new(0.0, 3.0, 2.0),
        Vector3::new(6.1, 8.0, 9.0),
        Vector3::new(2.0, 1.0, -2.0),
        Vector3::new(5.2, 2.0, -1.0),
        Vector3::new(7.0, -3.0, -0.15),
        Vector3::new(0.0, 0.0, 0.0001),
        Vector3::new(0.0, 0.0, 0.000001),
    ];
    let test_reals: [real; 5] = [1.0, 0.0005, 0.000005, 0.0000001, 0.0];
    let test_inf_planes = [
        Plane {
            normal: Vector3::new(real::INFINITY, 0.0, 0.0),
            d: 10.0,
        },
        Plane {
            normal: Vector3::new(real::NAN, real::INFINITY, real::NEG_INFINITY),
            d: real::NAN,
        },
        Plane {
            normal: Vector3::new(0.8, real::INFINITY, -1.2),
            d: 3.5,
        },
    ];

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
        assert_eq_approx!(
            outer,
            inner,
            |a: Plane, b: Plane| a.is_equal_approx(&b),
            "{context}: outer != inner ({outer:?} != {inner:?})"
        );
    }

    for a in unnormalized_planes {
        let inner_a = InnerPlane::from_outer(&a);
        check_mapping_eq_approx_plane("normalized", a.normalized(), inner_a.normalized());
    }

    for a in test_planes {
        let inner_a = InnerPlane::from_outer(&a);

        check_mapping_eq("center", a.center(), inner_a.get_center());
        check_mapping_eq("is_finite", a.is_finite(), inner_a.is_finite());

        for b in test_inf_planes {
            let inner_b = InnerPlane::from_outer(&b);
            check_mapping_eq("is_finite", b.is_finite(), inner_b.is_finite());
        }

        for b in test_vectors {
            check_mapping_eq(
                "distance_to",
                a.distance_to(b).as_f64(),
                inner_a.distance_to(b),
            );
            check_mapping_eq(
                "is_point_over",
                a.is_point_over(b),
                inner_a.is_point_over(b),
            );
            check_mapping_eq("project", a.project(b), inner_a.project(b));
            for c in test_vectors {
                check_mapping_eq(
                    "intersect_segment",
                    a.intersect_segment(b, c)
                        .as_ref()
                        .map(ToVariant::to_variant)
                        .unwrap_or_default(),
                    inner_a.intersects_segment(b, c),
                );
                check_mapping_eq(
                    "intersect_ray",
                    a.intersect_ray(b, c)
                        .as_ref()
                        .map(ToVariant::to_variant)
                        .unwrap_or_default(),
                    inner_a.intersects_ray(b, c),
                );
            }
            for c in test_reals {
                check_mapping_eq(
                    "contains_point",
                    a.contains_point(b, Some(c)),
                    inner_a.has_point(b, c.as_f64()),
                );
            }
        }

        for b in test_planes {
            check_mapping_eq(
                "is_equal_approx",
                a.is_equal_approx(&b),
                inner_a.is_equal_approx(b),
            );
            for c in test_planes {
                check_mapping_eq(
                    "intersect_3",
                    a.intersect_3(&b, &c)
                        .as_ref()
                        .map(ToVariant::to_variant)
                        .unwrap_or_default(),
                    inner_a.intersect_3(b, c),
                );
            }
        }
    }
}
