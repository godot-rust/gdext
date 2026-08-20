/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::hint::black_box;

use godot::builtin::inner::InnerRect2i;
use godot::builtin::{GString, GodotStringExt, PackedInt32Array, Rect2i, StringName, Vector2i};

use crate::framework::{BenchResult, bench, bench_measure};

#[bench]
fn builtin_gstring_ctor() -> GString {
    "some test string".to_gstring()
}

#[bench]
fn builtin_stringname_ctor() -> StringName {
    "some test string".to_string_name()
}

#[bench(manual)]
fn builtin_gstring_to_rust() -> BenchResult {
    let string = "some test string".to_gstring();

    bench_measure(|| String::from(&string))
}

#[bench]
fn builtin_rust_call() -> bool {
    let point = black_box(Vector2i::new(50, 60));

    let rect = Rect2i::from_components(0, 0, 100, 100);

    rect.contains_point(point)
}

#[bench]
fn builtin_ffi_call() -> bool {
    let point = black_box(Vector2i::new(50, 60));

    let rect = Rect2i::from_components(0, 0, 100, 100);
    let rect = InnerRect2i::from_outer(&rect);

    rect.has_point(point)
}

#[bench]
fn utilities_allocate_rid() -> i64 {
    godot::global::rid_allocate_id()
}

#[bench]
fn utilities_rust_call() -> f64 {
    let base = black_box(5.678);
    let exponent = black_box(3.456);

    f64::powf(base, exponent)
}

#[bench]
fn utilities_ffi_call() -> f64 {
    let base = black_box(5.678);
    let exponent = black_box(3.456);

    godot::global::pow(base, exponent)
}

#[bench]
fn packed_from_iter_sized() -> PackedInt32Array {
    // Create an iterator whose `size_hint()` returns `(len, Some(len))`.
    PackedInt32Array::from_iter(0..100)
}

#[bench]
fn packed_from_iter_unsized() -> PackedInt32Array {
    // Create an iterator whose `size_hint()` returns `(0, None)`.
    let mut item = 0;
    PackedInt32Array::from_iter(std::iter::from_fn(|| {
        item += 1;
        if item <= 100 { Some(item) } else { None }
    }))
}
