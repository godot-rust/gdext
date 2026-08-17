/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Variant marshalling benchmarks.
//!
//! One bench per code path, not per type: all `RustMarshal` types share the same generic memcpy path, so `i64` (8 bytes) and `Vector3`
//! (12 bytes, depends on `real` type) cover it. Per-type correctness is verified in `builtin_tests::containers::rust_variant_test`.
//!
//! Each bench uses 1000 inner repetitions, since single operations are far below the measurement noise floor (see `framework::bencher`).

use std::hint::black_box;

use godot::builtin::{GString, Vector3};
use godot::meta::{FromGodot, ToGodot};

use crate::framework::{BenchResult, bench, bench_measure};

/// POD fast path: 8-byte payload, construction + read-back + drop.
#[bench(repeat = 1000)]
fn variant_roundtrip_i64() -> i64 {
    let v = black_box(12345_i64).to_variant();
    i64::from_variant(black_box(&v))
}

/// POD fast path: 12-byte payload, depends on `real` type precision.
#[bench(repeat = 1000)]
fn variant_roundtrip_vector3() -> Vector3 {
    let v = black_box(Vector3::new(1.5, 2.5, 3.5)).to_variant();
    Vector3::from_variant(black_box(&v))
}

/// Control for the FFI path: refcounted types must not regress from the POD dispatch.
#[bench(manual)]
fn variant_roundtrip_gstring() -> BenchResult {
    let s = GString::from("hello world");

    bench_measure(1000, || {
        let v = black_box(&s).to_variant();
        GString::from_variant(black_box(&v))
    })
}

/// `Clone` has its own fast path (bytewise copy instead of `variant_new_copy`), not covered by the roundtrips.
#[bench(manual)]
fn variant_clone_vector3() -> BenchResult {
    let v = Vector3::new(1.0, 2.0, 3.0).to_variant();

    bench_measure(1000, || black_box(&v).clone())
}
