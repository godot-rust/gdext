/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::builtin::Variant;
use godot::prelude::{varray, Callable, RustCallable};

use crate::framework::{bench, bench_measure, BenchResult};

#[bench(manual)]
fn callable_callv_rust_fn() -> BenchResult {
    let callable = Callable::from_fn("RustFunction", |_| ());
    let arg = varray![];

    bench_measure(25, || callable.callv(&arg))
}

#[bench(manual)]
fn callable_callv_custom() -> BenchResult {
    let callable = Callable::from_custom(MyRustCallable {});
    let arg = varray![];

    bench_measure(25, || callable.callv(&arg))
}

#[bench(manual)]
fn callable_to_string_rust_fn() -> BenchResult {
    let callable = Callable::from_fn("RustFunction", |_| ());

    bench_measure(1000, || callable.to_string())
}

#[bench(manual)]
fn callable_to_string_custom() -> BenchResult {
    let callable = Callable::from_custom(MyRustCallable {});

    bench_measure(1000, || callable.to_string())
}

// Helpers for benchmarks above

#[derive(PartialEq, Hash)]
struct MyRustCallable {}

impl RustCallable for MyRustCallable {
    fn invoke(&mut self, _args: &[&Variant]) -> Variant {
        Variant::nil()
    }
}

impl std::fmt::Display for MyRustCallable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MyRustCallable")
    }
}
