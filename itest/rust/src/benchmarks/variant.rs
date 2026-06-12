/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::hint::black_box;

use godot::builtin::{Color, Rid, Variant, Vector2i, Vector3};
use godot::meta::{FromGodot, ToGodot};

use crate::framework::{BenchResult, bench, bench_measure};

// Scalar types.

#[bench]
fn variant_from_bool() -> Variant {
    black_box(true).to_variant()
}

#[bench]
fn variant_to_bool() -> bool {
    bool::from_variant(black_box(&true.to_variant()))
}

#[bench]
fn variant_from_i64() -> Variant {
    black_box(12345_i64).to_variant()
}

#[bench]
fn variant_to_i64() -> i64 {
    i64::from_variant(black_box(&12345_i64.to_variant()))
}

#[bench]
fn variant_from_f64() -> Variant {
    black_box(1.234_f64).to_variant()
}

#[bench]
fn variant_to_f64() -> f64 {
    f64::from_variant(black_box(&1.234_f64.to_variant()))
}

// Vector types.

#[bench]
fn variant_from_vector2i() -> Variant {
    black_box(Vector2i::new(100, 200)).to_variant()
}

#[bench]
fn variant_to_vector2i() -> Vector2i {
    Vector2i::from_variant(black_box(&Vector2i::new(100, 200).to_variant()))
}

#[bench]
fn variant_from_vector3() -> Variant {
    black_box(Vector3::new(1.5, 2.5, 3.5)).to_variant()
}

#[bench]
fn variant_to_vector3() -> Vector3 {
    Vector3::from_variant(black_box(&Vector3::new(1.5, 2.5, 3.5).to_variant()))
}

// Other POD types.

#[bench]
fn variant_from_color() -> Variant {
    black_box(Color::from_rgba(0.5, 0.3, 0.8, 1.0)).to_variant()
}

#[bench]
fn variant_to_color() -> Color {
    Color::from_variant(black_box(
        &Color::from_rgba(0.5, 0.3, 0.8, 1.0).to_variant(),
    ))
}

#[bench]
fn variant_from_rid() -> Variant {
    black_box(Rid::new(12345)).to_variant()
}

#[bench]
fn variant_to_rid() -> Rid {
    Rid::from_variant(black_box(&Rid::new(12345).to_variant()))
}

// Lifecycle: nil construction, clone, drop.

#[bench]
fn variant_nil_ctor() -> Variant {
    Variant::nil()
}

#[bench]
fn variant_clone_i64() -> Variant {
    let v = black_box(12345_i64).to_variant();
    black_box(&v).clone()
}

#[bench]
fn variant_clone_vector3() -> Variant {
    let v = black_box(Vector3::new(1.0, 2.0, 3.0)).to_variant();
    black_box(&v).clone()
}

#[bench(manual)]
fn variant_drop_i64_x1000() -> BenchResult {
    bench_measure(1, || {
        let mut count = 0_i64;
        for i in 0..1000_i64 {
            let v = black_box(i).to_variant();
            count += 1;
            drop(black_box(v));
        }
        black_box(count)
    })
}

#[bench(manual)]
fn variant_drop_vector3_x1000() -> BenchResult {
    bench_measure(1, || {
        let mut count = 0_i32;
        for i in 0..1000_i32 {
            let v = black_box(Vector3::new(i as f32, i as f32, i as f32)).to_variant();
            count += 1;
            drop(black_box(v));
        }
        black_box(count)
    })
}

// Bulk round-trips (representative of real use: many values through variants).

#[bench(manual)]
fn variant_roundtrip_i64_x1000() -> BenchResult {
    bench_measure(1, || {
        let mut sum = 0i64;
        for i in 0..1000_i64 {
            let v = black_box(i).to_variant();
            sum = sum.wrapping_add(i64::from_variant(black_box(&v)));
        }
        black_box(sum)
    })
}

#[bench(manual)]
fn variant_roundtrip_vector3_x1000() -> BenchResult {
    bench_measure(1, || {
        let mut result = Vector3::ZERO;
        for i in 0..1000_i32 {
            let v = black_box(Vector3::new(i as f32, i as f32 * 2.0, i as f32 * 3.0)).to_variant();
            result += Vector3::from_variant(black_box(&v));
        }
        black_box(result)
    })
}
