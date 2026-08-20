/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Variant marshalling benchmarks.
//!
//! One bench per code path, not per type: `i64` (smallest POD), `Vector4` (largest POD that still fits inline), `Aabb` (smallest type
//! that overflows inline storage), plus `GString`/`Gd` as refcounted controls.

use std::hint::black_box;

use godot::builtin::{
    Aabb, Array, Dictionary, GString, GodotStringExt, PackedInt32Array, VarArray, Variant, Vector3,
    Vector4, real,
};
use godot::classes::RefCounted;
use godot::meta::{FromGodot, ToGodot};
use godot::obj::{Gd, NewGd};

use crate::framework::{BenchResult, bench, bench_measure, bench_measure_batched};

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Roundtrip: to_variant() + from_variant().

#[bench]
fn variant_roundtrip_i64() -> i64 {
    let v = black_box(12345_i64).to_variant();
    i64::from_variant(black_box(&v))
}

#[bench]
fn variant_roundtrip_vector4() -> Vector4 {
    let v = black_box(Vector4::new(1.5, 2.5, 3.5, 4.5)).to_variant();
    Vector4::from_variant(black_box(&v))
}

#[bench]
fn variant_roundtrip_aabb() -> Aabb {
    let v = black_box(Aabb::new(
        Vector3::new(1.0, 2.0, 3.0),
        Vector3::new(4.0, 5.0, 6.0),
    ))
    .to_variant();
    Aabb::from_variant(black_box(&v))
}

/// Control: refcounted payload, which cannot bypass the Variant FFI.
#[bench(manual)]
fn variant_roundtrip_gstring() -> BenchResult {
    let s = "hello world".to_gstring();

    bench_measure(|| {
        let v = black_box(&s).to_variant();
        GString::from_variant(black_box(&v))
    })
}

/// ~10x slower than [`variant_roundtrip_gstring()`]: object variants additionally do an instance-DB lookup on read-back.
#[bench(manual)]
fn variant_roundtrip_object() -> BenchResult {
    let obj = RefCounted::new_gd();

    bench_measure(|| {
        let v = black_box(&obj).to_variant();
        Gd::<RefCounted>::from_variant(black_box(&v))
    })
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Clone: copy + destroy, not covered by the roundtrips.

#[bench(manual)]
fn variant_clone_i64() -> BenchResult {
    let v = 12345_i64.to_variant();

    bench_measure(|| black_box(&v).clone())
}

#[bench(manual)]
fn variant_clone_vector4() -> BenchResult {
    let v = Vector4::new(1.0, 2.0, 3.0, 4.0).to_variant();

    bench_measure(|| black_box(&v).clone())
}

#[bench(manual)]
fn variant_clone_aabb() -> BenchResult {
    let v = Aabb::new(Vector3::new(1.0, 2.0, 3.0), Vector3::new(4.0, 5.0, 6.0)).to_variant();

    bench_measure(|| black_box(&v).clone())
}

/// Control: refcounted payload, so copy + destroy go through refcount inc/dec.
#[bench(manual)]
fn variant_clone_object() -> BenchResult {
    let v = RefCounted::new_gd().to_variant();

    bench_measure(|| black_box(&v).clone())
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Construct + drop: isolates construction/destruction from the read-back step in the roundtrips above.

#[bench]
fn variant_ctor_drop_i64() -> Variant {
    black_box(12345_i64).to_variant()
}

#[bench]
fn variant_ctor_drop_vector4() -> Variant {
    black_box(Vector4::new(1.5, 2.5, 3.5, 4.5)).to_variant()
}

#[bench]
fn variant_ctor_drop_aabb() -> Variant {
    black_box(Aabb::new(
        Vector3::new(1.0, 2.0, 3.0),
        Vector3::new(4.0, 5.0, 6.0),
    ))
    .to_variant()
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Drop: consumes its input, so the variants are built outside the timed window.

#[bench(manual)]
fn variant_drop_i64() -> BenchResult {
    bench_measure_batched(
        |i| (i as i64).to_variant(),
        |v| v, // Dropped inside the measured loop.
    )
}

#[bench(manual)]
fn variant_drop_aabb() -> BenchResult {
    bench_measure_batched(
        |i| {
            let f = i as real;
            Aabb::new(Vector3::new(f, f, f), Vector3::new(f, f, f)).to_variant()
        },
        |v| v,
    )
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Default: NIL variant construction.

#[bench]
fn variant_default() -> Variant {
    Variant::default()
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// get_type(): tag read for most types; OBJECT pays an extra FFI call to detect null instances.

#[bench(manual)]
fn variant_get_type_i64() -> BenchResult {
    let v = 12345_i64.to_variant();

    bench_measure(|| black_box(&v).get_type())
}

#[bench(manual)]
fn variant_get_type_object() -> BenchResult {
    let v = RefCounted::new_gd().to_variant();

    bench_measure(|| black_box(&v).get_type())
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Conversion failure: error path, where type checks and diagnostics can allocate.

// ~2x a successful `i64` roundtrip, i.e. the error path allocates.
#[bench(manual)]
fn variant_try_from_mismatched() -> BenchResult {
    let v = "hello world".to_gstring().to_variant();

    bench_measure(|| i64::try_from_variant(black_box(&v)).is_err())
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Combined: array/packed-array conversions, which marshal every element through Variant.

#[bench]
fn array_from_iter_i32() -> Array<i32> {
    Array::<i32>::from_iter(0..100)
}

#[bench(manual)]
fn array_to_vec_i32() -> BenchResult {
    let array = Array::<i32>::from_iter(0..100);

    bench_measure(|| array.iter_shared().collect::<Vec<i32>>())
}

/// Not natively supported by Godot, unlike `packed_to_var_array_i32()` below.
#[bench(manual)]
fn packed_to_array_i32() -> BenchResult {
    let packed = PackedInt32Array::from_iter(0..100);

    bench_measure(|| packed.to_typed_array())
}

/// Control: native Godot conversion, no per-element marshalling.
#[bench(manual)]
fn packed_to_varray_i32() -> BenchResult {
    let packed = PackedInt32Array::from_iter(0..100);

    bench_measure(|| packed.to_var_array())
}

// ~1.5x slower than array_to_vec_i32() -> converting to typed isn't just
#[bench(manual)]
fn varray_to_vec_i32() -> BenchResult {
    let var_array: VarArray = (0..100).map(|i: i32| i.to_variant()).collect();

    bench_measure(|| {
        var_array
            .iter_shared()
            .map(|v| i32::from_variant(&v))
            .collect::<Vec<i32>>()
    })
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Combined: typed Dictionary from_iter + readback.

#[bench]
fn dict_from_iter_i64() -> Dictionary<i64, i64> {
    Dictionary::<i64, i64>::from_iter((0..100).map(|i| (i, i * 2)))
}

#[bench(manual)]
fn dict_to_vec_i64() -> BenchResult {
    let dict = Dictionary::<i64, i64>::from_iter((0..100).map(|i| (i, i * 2)));

    bench_measure(|| dict.iter_shared().collect::<Vec<(i64, i64)>>())
}
