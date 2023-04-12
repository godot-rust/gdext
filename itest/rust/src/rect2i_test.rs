/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
use std::fmt::Debug;

use crate::itest;
use godot::prelude::{inner::InnerRect2i, *};

#[itest]
fn rect2i_equiv_unary() {
    let test_rects = [
        Rect2i::from_components(0, 0, 1, 0),
        Rect2i::from_components(0, 0, 1, 1),
        Rect2i::from_components(0, 0, 10, 10),
        Rect2i::from_components(4, 4, 1, 1),
        Rect2i::from_components(8, 8, 2, 2),
        Rect2i::from_components(8, 8, 2, 3),
    ];
    let test_vectors = [
        Vector2i::ZERO,
        Vector2i::new(0, 10),
        Vector2i::new(10, 0),
        Vector2i::new(10, 10),
    ];
    let test_ints = [0, 1, 10, 32];
    let test_sides = [
        RectSide::Left,
        RectSide::Top,
        RectSide::Right,
        RectSide::Bottom,
    ];

    fn evaluate_mappings<T>(key: &str, a: T, b: T)
    where
        T: Eq + Debug,
    {
        assert_eq!(a, b, "{}: outer != inner ({:?} != {:?})", key, a, b);
    }

    for a in test_rects {
        let inner_a = InnerRect2i::from_outer(&a);

        evaluate_mappings("abs", a.abs(), inner_a.abs());
        evaluate_mappings("area", a.area(), inner_a.get_area() as i32);
        evaluate_mappings("center", a.center(), inner_a.get_center());
        evaluate_mappings("has_area", a.has_area(), inner_a.has_area());

        for b in test_rects {
            evaluate_mappings("encloses", a.encloses(b), inner_a.encloses(b));
            evaluate_mappings("intersects", a.intersects(b), inner_a.intersects(b));
            evaluate_mappings(
                "intersection",
                a.intersection(b).unwrap_or_default(),
                inner_a.intersection(b),
            );
            evaluate_mappings("merge", a.merge(b), inner_a.merge(b));
        }

        for b in test_vectors {
            evaluate_mappings("expand", a.expand(b), inner_a.expand(b));
            evaluate_mappings("contains_point", a.contains_point(b), inner_a.has_point(b));
        }

        for b in test_ints {
            evaluate_mappings("grow", a.grow(b), inner_a.grow(b as i64));

            for c in test_ints {
                for d in test_ints {
                    for e in test_ints {
                        evaluate_mappings(
                            "grow_individual",
                            a.grow_individual(b, c, d, e),
                            inner_a.grow_individual(b as i64, c as i64, d as i64, e as i64),
                        );
                    }
                }
            }
        }

        for b in test_sides {
            for c in test_ints {
                evaluate_mappings(
                    "grow_side",
                    a.grow_side(b, c),
                    inner_a.grow_side(b as i64, c as i64),
                );
            }
        }
    }
}
