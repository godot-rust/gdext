/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
use std::fmt::Debug;

use crate::itest;
use godot::{
    builtin::{Rect2, RectSide, Vector2},
    prelude::inner::InnerRect2,
};

#[itest]
fn rect2_equiv_unary() {
    let test_rects = [
        Rect2::from_components(0.2, 0.3, 1.5, 0.9),
        Rect2::from_components(0.2, 0.3, 1.5, 1.9),
        Rect2::from_components(0.2, 0.3, 1.0, 1.9),
        Rect2::from_components(4.2, 4.3, 1.5, 1.9),
        Rect2::from_components(8.2, 8.3, 2.5, 2.9),
        Rect2::from_components(8.2, 8.3, 2.5, 3.9),
    ];
    let test_vectors = [
        Vector2::ZERO,
        Vector2::new(0.0, 10.0),
        Vector2::new(10.0, 0.0),
        Vector2::new(10.0, 10.0),
    ];
    let test_reals = [0.0, 1.0, 10.0, 32.0];
    let grow_values = [-1.0, 0.0, 1.0, 7.0];
    let test_sides = [
        RectSide::Left,
        RectSide::Top,
        RectSide::Right,
        RectSide::Bottom,
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

    for a in test_rects {
        let inner_a = InnerRect2::from_outer(&a);

        check_mapping_eq("abs", a.abs(), inner_a.abs());
        check_mapping_eq("area", a.area() as f64, inner_a.get_area());
        check_mapping_eq("center", a.center(), inner_a.get_center());
        check_mapping_eq("has_area", a.has_area(), inner_a.has_area());

        for b in test_rects {
            check_mapping_eq("encloses", a.encloses(b), inner_a.encloses(b));
            check_mapping_eq(
                "intersects",
                a.intersects(b, true),
                inner_a.intersects(b, true),
            );
            // Check intersection without considering borders
            check_mapping_eq(
                "intersects",
                a.intersects(b, false),
                inner_a.intersects(b, false),
            );
            check_mapping_eq(
                "intersection",
                a.intersection(b).unwrap_or_default(),
                inner_a.intersection(b),
            );
            check_mapping_eq("merge", a.merge(b), inner_a.merge(b));
        }

        for b in test_vectors {
            check_mapping_eq("expand", a.expand(b), inner_a.expand(b));
            check_mapping_eq("has_point", a.has_point(b), inner_a.has_point(b));
        }

        for b in grow_values {
            check_mapping_eq("grow", a.grow(b as f32), inner_a.grow(b));

            for c in grow_values {
                for d in grow_values {
                    for e in grow_values {
                        check_mapping_eq(
                            "grow_individual",
                            a.grow_individual(b as f32, c as f32, d as f32, e as f32),
                            inner_a.grow_individual(b, c, d, e),
                        );
                    }
                }
            }
        }

        for b in test_sides {
            for c in test_reals {
                check_mapping_eq(
                    "grow_side",
                    a.grow_side(b, c as f32),
                    inner_a.grow_side(b as i64, c),
                );
            }
        }
    }
}
