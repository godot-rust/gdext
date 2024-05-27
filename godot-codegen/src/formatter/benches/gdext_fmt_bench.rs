/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// IMPORTANT: to enable this benchmark, uncomment the corresponding lines in Cargo.toml.
// First tried with #![allow(clippy::all)], but clippy still tries to compile the code and fails on imports.
#![cfg(FALSE)]

use std::{path::PathBuf, str::FromStr};

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use godot_fmt::format_tokens;
use proc_macro2::TokenStream;

pub fn criterion_benchmark(c: &mut Criterion) {
    let test_cases_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("godot-codegen")
        .join("src")
        .join("formatter")
        .join("test-cases");

    for dir_entry in std::fs::read_dir(test_cases_dir).unwrap() {
        let dir_entry = dir_entry.unwrap();
        let path = dir_entry.path();

        let contents = std::fs::read_to_string(&path).unwrap();
        let stream = TokenStream::from_str(&contents).unwrap();

        let name = path.file_stem().unwrap().to_str().unwrap();
        c.bench_function(name, move |b| {
            b.iter(|| format_tokens(black_box(stream.clone())))
        });
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
