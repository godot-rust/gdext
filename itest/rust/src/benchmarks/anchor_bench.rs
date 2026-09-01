/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Benchmarks over pure Rust code, ranging from sub-nanosecond (integer multiply) to 100s-of-nanoseconds (hashmap build), covering similar
//! range to Godot benchmarks. Their duration cannot change between commits, so whatever they move by is measurement error. The runner finds them
//! through the `anchor_` prefix and reports their spread as _noise floor_.
//!
//! Such a floor covers one run. Two builds may also differ in code layout, which shifts anchors' absolute times -- comparing those is manual.

use std::collections::HashMap;
use std::hash::{BuildHasherDefault, DefaultHasher};
use std::hint::black_box;

use crate::framework::bench;

#[bench]
fn anchor_int_mul() -> u64 {
    black_box(0x9e3779b97f4a7c15u64).wrapping_mul(black_box(6364136223846793005))
}

#[bench]
fn anchor_vec_push() -> Vec<u32> {
    let mut vec = Vec::with_capacity(16);
    for i in 0..16u32 {
        vec.push(black_box(i));
    }
    vec
}

#[bench]
fn anchor_hashmap_build() -> Option<u32> {
    // Rebuilt per operation on purpose: a lookup alone is too fast to separate from the timer, so construction dominates.
    // The hasher is fixed, as the per-process seed of `RandomState` would move this anchor for reasons other than the machine.
    let map: HashMap<u32, u32, BuildHasherDefault<DefaultHasher>> =
        (0..32u32).map(|i| (i, i * 2)).collect();

    map.get(&black_box(17)).copied()
}

#[bench]
fn anchor_string_format() -> String {
    format!("{}-{}", black_box(42), black_box("anchor"))
}
