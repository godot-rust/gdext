/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// File can be split once this grows.

use std::hint::black_box;

use godot::bind::GodotClass;
use godot::builtin::inner::InnerRect2i;
use godot::builtin::{GString, Rect2i, StringName, Vector2i};
use godot::engine::{Node3D, Os, RefCounted};
use godot::obj::{Gd, InstanceId};

use crate::framework::bench;

#[bench]
fn builtin_string_ctor() -> GString {
    GString::from("some test string")
}

#[bench]
fn builtin_stringname_ctor() -> StringName {
    StringName::from("some test string")
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

#[bench(repeat = 25)]
fn class_node_life() -> InstanceId {
    let node = Node3D::new_alloc();
    let instance_id = node.instance_id();

    node.free();
    instance_id // No longer valid, but enough for compiler to assume it's used.
}

#[bench(repeat = 25)]
fn class_refcounted_life() -> Gd<RefCounted> {
    RefCounted::new()
}

#[bench(repeat = 25)]
fn class_user_refc_life() -> Gd<MyBenchType> {
    Gd::default()
}

#[bench]
fn class_singleton_access() -> Gd<Os> {
    Os::singleton()
}

#[bench]
fn utilities_allocate_rid() -> i64 {
    godot::engine::utilities::rid_allocate_id()
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

    godot::engine::utilities::pow(base, exponent)
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Helpers for benchmarks above

#[derive(GodotClass)]
#[class(init)]
struct MyBenchType {}
