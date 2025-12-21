/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::builtin::GString;

use crate::framework::{bench, bench_measure, BenchResult};

#[bench(manual)]
fn gstring_find_ffi() -> BenchResult {
    let string = GString::from("the quick brown fox jumps over the lazy dog");

    bench_measure(1000, || {
        let index = string.find("fox");
        assert_eq!(index, Some(16));
        index
    })
}

#[bench(manual)]
fn gstring_find_rust() -> BenchResult {
    let string = GString::from("the quick brown fox jumps over the lazy dog");

    bench_measure(1000, || {
        // No allocation: scan bytes and return the start index.
        let index = find_in_chars(string.chars(), "fox");

        assert_eq!(index, Some(16));

        index
    })
}

fn find_in_chars(hay: &[char], needle: &str) -> Option<usize> {
    // Match Rust's `str::find` behavior: empty needle matches at 0.
    if needle.is_empty() {
        return Some(0);
    }

    // Count needle length in chars (no allocation).
    let n = needle.chars().count();
    if n > hay.len() {
        return None;
    }

    'outer: for i in 0..=hay.len() - n {
        let mut it = needle.chars(); // re-decode each attempt; still no allocation
        for j in 0..n {
            // Safe unwrap: we know `it` has exactly `n` chars.
            if hay[i + j] != it.next().unwrap() {
                continue 'outer;
            }
        }
        return Some(i);
    }

    None
}
