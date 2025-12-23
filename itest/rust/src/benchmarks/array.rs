/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::hint::black_box;

use godot::builtin::{Array, GString};

use crate::framework::{bench, bench_measure, BenchResult};

#[bench(manual)]
fn array_extend_i64_standard() -> BenchResult {
    bench_measure(1, || {
        let mut arr = Array::new();
        arr.extend((10_000i64..20_000).map(black_box));
        black_box(arr)
    })
}

#[bench(manual)]
fn array_extend_i64_blaze() -> BenchResult {
    bench_measure(1, || {
        let mut arr = Array::new();
        arr.extend_blaze((10_000i64..20_000).map(black_box));
        black_box(arr)
    })
}

#[bench(manual)]
fn array_extend_gstring_standard() -> BenchResult {
    bench_measure(1, || {
        let mut arr = Array::new();
        arr.extend(
            (10_000..20_000).map(|i| black_box(GString::from(format!("str_{}", i).as_str()))),
        );
        black_box(arr)
    })
}

#[bench(manual)]
fn array_extend_gstring_blaze() -> BenchResult {
    bench_measure(1, || {
        let mut arr = Array::new();
        arr.extend_blaze(
            (10_000..20_000).map(|i| black_box(GString::from(format!("str_{}", i).as_str()))),
        );
        black_box(arr)
    })
}
