/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
use godot::builtin::inner::InnerAabb;
use godot::builtin::math::assert_eq_approx;
use godot::builtin::{Aabb, Plane, Vector3};

use crate::framework::itest;

const SAMPLE_AABB: Aabb = Aabb::new(Vector3::new(-1.5, 0.0, 2.0), Vector3::new(3.0, 2.0, 4.0));

#[itest]
fn aabb_equiv() {
    let inner = InnerAabb::from_outer(&SAMPLE_AABB);
    let outer = SAMPLE_AABB;
    let test_aabb = Aabb::new(
        SAMPLE_AABB.position - Vector3::new(1.0, 0.25, 0.55),
        SAMPLE_AABB.size,
    );

    #[rustfmt::skip]
    let mappings_aabb = [
        (
            "abs",
            inner.abs(),
            outer.abs()
        ),
        (
            "grow",
            inner.grow(4.0),
            outer.grow(4.0)
        ),
        (
            "intersection",
            inner.intersection(test_aabb),
            outer.intersect(test_aabb).expect("Must intersect"),
        ),
        (
            "interpolate_with",
            inner.merge(test_aabb),
            outer.merge(test_aabb),
        ),
    ];

    for (name, inner, outer) in mappings_aabb {
        assert_eq_approx!(inner, outer, "function: {name}\n");
    }

    // Check endpoints as well.
    for i in 0..8 {
        assert_eq_approx!(
            inner.get_endpoint(i as i64),
            outer.get_corner(i),
            "index: {i}\n"
        );
    }

    let intersecting_segment = (Vector3::new(-2.6, -1.0, 2.0), Vector3::new(3.0, 2.0, 2.0));
    let non_intersecting_segment = (Vector3::new(-4.5, 0.0, 2.0), Vector3::new(-2.5, 0.0, 2.0));

    #[rustfmt::skip]
    let mappings_vector3 = [
        (
            "center",
            inner.get_center(),
            outer.center()
        ),
        (
            "longest_axis",
            inner.get_longest_axis(),
            outer.longest_axis().expect("Must have the longest axis"),
        ),
        (
            "shortest_axis",
            inner.get_shortest_axis(),
            outer.shortest_axis().expect("Must have the shortest axis"),
        ),
        (
            "support",
            inner.get_support(Vector3::UP),
            outer.get_support(Vector3::UP),
        ),
        (
            "intersect_segment",
            inner.intersects_segment(intersecting_segment.0, intersecting_segment.1).try_to().expect("Failed to intersect segment!"),
            outer.intersect_segment(intersecting_segment.0, intersecting_segment.1).expect(" Failed to intersect segment!"),
        ),
    ];

    for (name, inner, outer) in mappings_vector3 {
        assert_eq_approx!(inner, outer, "function: {name}\n");
    }

    let enclosed = SAMPLE_AABB.grow(-1.0);
    let test_plane = Plane::new(Vector3::UP, 0.0);

    #[rustfmt::skip]
    let mappings_bool = [
        (
            "encloses",
            inner.encloses(enclosed),
            outer.encloses(enclosed),
        ),
        (
            "has_surface",
            inner.has_surface(),
            outer.has_surface()
        ),
        (
            "has_volume",
            inner.has_volume(),
            outer.has_volume()
        ),
        (
            "intersects",
            inner.intersects(enclosed),
            outer.intersects(enclosed),
        ),
        (
            "intersects_plane",
            inner.intersects_plane(test_plane),
            outer.intersects_plane(test_plane),
        ),
        (
            "intersects_segment",
            !inner.intersects_segment(intersecting_segment.0, intersecting_segment.1).is_nil(),
            outer.intersects_segment(intersecting_segment.0, intersecting_segment.1),
        ),
        (
            "intersect_segment",
            inner.intersects_segment(non_intersecting_segment.0, non_intersecting_segment.1).is_nil(),
            outer.intersect_segment(non_intersecting_segment.0, non_intersecting_segment.1).is_none(),
        ),
        (
            "is_finite",
            inner.is_finite(),
            outer.is_finite()
        ),
    ];

    for (name, inner, outer) in mappings_bool {
        assert_eq!(inner, outer, "function: {name}\n");
    }
}
