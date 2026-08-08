/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::hint::black_box;

use godot::builtin::{Array, GodotStringExt};

use crate::framework::{BenchResult, bench, bench_measure};

#[bench(manual)]
fn array_extend_i64() -> BenchResult {
    bench_measure(1, || {
        let mut arr = Array::new();
        arr.extend((10_000i64..20_000).map(black_box));
        black_box(arr)
    })
}

#[bench(manual)]
fn array_from_iter_i64() -> BenchResult {
    bench_measure(1, || {
        black_box((10_000i64..20_000).map(black_box).collect::<Array<i64>>())
    })
}

#[bench(manual)]
fn array_iter_i64() -> BenchResult {
    let arr: Array<i64> = (0i64..10_000).collect();
    bench_measure(1, move || {
        black_box(arr.iter_shared().map(black_box).sum::<i64>())
    })
}

#[bench(manual)]
fn array_resize_i64() -> BenchResult {
    bench_measure(1, || {
        let mut arr = Array::<i64>::new();
        arr.resize(10_000, black_box(123i64));
        black_box(arr)
    })
}

#[bench(manual)]
fn array_resize_gstring() -> BenchResult {
    let value = "str".to_gstring();
    bench_measure(1, move || {
        let mut arr = Array::<godot::builtin::GString>::new();
        arr.resize(10_000, black_box(&value));
        black_box(arr)
    })
}

#[bench(manual)]
fn array_extend_gstring() -> BenchResult {
    bench_measure(1, || {
        let mut arr = Array::new();
        arr.extend((10_000..20_000).map(|i| black_box(format!("str_{}", i).to_gstring())));
        black_box(arr)
    })
}
