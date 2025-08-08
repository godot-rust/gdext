/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::builtin::inner::InnerRect2;
use godot::builtin::math::assert_eq_approx;
use godot::builtin::{real, reals, RealConv, Rect2, Side, Vector2};

use crate::framework::itest;

#[itest]
fn rect2_inner_equivalence() {
    let rects = [
        Rect2::from_components(0.2, 0.3, 1.5, 0.9),
        Rect2::from_components(0.2, 0.3, 1.5, 1.9),
        Rect2::from_components(0.2, 0.3, 1.0, 1.9),
        Rect2::from_components(4.2, 4.3, 1.5, 1.9),
        Rect2::from_components(8.2, 8.3, 2.5, 2.9),
        Rect2::from_components(8.2, 8.3, 2.5, 3.9),
    ];
    let vectors = [
        Vector2::ZERO,
        Vector2::new(0.0, 10.0),
        Vector2::new(10.0, 0.0),
        Vector2::new(10.0, 10.0),
    ];
    let reals = reals![0.0, 1.0, 32.0];
    let grow_values = reals![-1.0, 0.0, 7.0];
    let sides = [Side::LEFT, Side::TOP, Side::RIGHT, Side::BOTTOM];

    for rect in rects {
        let inner_rect = InnerRect2::from_outer(&rect);

        assert_eq_approx!(rect.abs(), inner_rect.abs());
        assert_eq_approx!(rect.area(), real::from_f64(inner_rect.get_area()));
        assert_eq_approx!(rect.center(), inner_rect.get_center());
        assert_eq!(rect.has_area(), inner_rect.has_area());

        for other in rects {
            assert_eq!(rect.encloses(other), inner_rect.encloses(other));
            assert_eq!(rect.intersects(other), inner_rect.intersects(other, true),);
            // Check intersection without considering borders
            assert_eq!(
                rect.intersects_exclude_borders(other),
                inner_rect.intersects(other, false),
            );
            assert_eq!(
                rect.intersect(other).unwrap_or_default(),
                inner_rect.intersection(other),
            );
            assert_eq_approx!(rect.merge(other), inner_rect.merge(other));
        }

        for vec in vectors {
            assert_eq_approx!(rect.expand(vec), inner_rect.expand(vec));
            assert_eq!(rect.contains_point(vec), inner_rect.has_point(vec));
        }

        for grow in grow_values {
            assert_eq_approx!(rect.grow(grow), inner_rect.grow(grow.as_f64()));
        }

        for left in grow_values {
            for top in grow_values {
                for right in grow_values {
                    for bottom in grow_values {
                        assert_eq_approx!(
                            rect.grow_individual(left, top, right, bottom),
                            inner_rect.grow_individual(
                                left.as_f64(),
                                top.as_f64(),
                                right.as_f64(),
                                bottom.as_f64(),
                            ),
                        );
                    }
                }
            }
        }

        for side in sides {
            for amount in reals {
                assert_eq_approx!(
                    rect.grow_side(side, amount),
                    inner_rect.grow_side(side as i64, amount.as_f64()),
                );
            }
        }
    }
}
