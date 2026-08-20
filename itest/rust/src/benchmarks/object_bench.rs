/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::builtin::GodotStringExt;
use godot::classes::{Node3D, Object, Os, RefCounted};
use godot::meta::ToGodot;
use godot::obj::{Gd, InstanceId, NewAlloc, NewGd, Singleton};

use super::BenchObj;
use crate::framework::{BenchResult, bench, bench_measure, create_gdscript};

#[bench]
fn class_node_life() -> InstanceId {
    let node = Node3D::new_alloc();
    let instance_id = node.instance_id();

    node.free();
    instance_id // No longer valid, but enough for compiler to assume it's used.
}

#[bench]
fn class_engine_refc_life() -> Gd<RefCounted> {
    RefCounted::new_gd()
}

#[bench]
fn class_user_refc_life() -> Gd<BenchObj> {
    BenchObj::new_gd()
}

/// Just measure `refc_inc` + `refc_dec` on already-existing object, without construction or destruction.
#[bench(manual)]
fn class_engine_refc_clone_drop() -> BenchResult {
    let obj = RefCounted::new_gd();

    bench_measure(|| obj.clone())
}

/// Same as [`class_engine_refc_clone_drop()`], for a user class -- `RawGd` additionally carries the storage-pointer cache.
#[bench(manual)]
fn class_user_refc_clone_drop() -> BenchResult {
    let obj = BenchObj::new_gd();

    bench_measure(|| obj.clone())
}

#[bench(manual)]
fn class_user_bind() -> BenchResult {
    let obj = BenchObj::new_gd();

    bench_measure(|| {
        let guard = obj.bind();
        std::hint::black_box(&guard);
        true
    })
}

#[bench(manual)]
fn class_user_bind_mut() -> BenchResult {
    let mut obj = BenchObj::new_gd();

    bench_measure(|| {
        let guard = obj.bind_mut();
        std::hint::black_box(&guard);
        true
    })
}

#[bench(manual)]
fn class_user_from_instance_id() -> BenchResult {
    let obj = BenchObj::new_gd();
    let instance_id = obj.instance_id();

    bench_measure(|| Gd::<BenchObj>::from_instance_id(instance_id))
}

/// Includes one `Gd` clone, since `try_cast()` consumes the object; see [`class_engine_refc_clone_drop()`].
#[bench(manual)]
fn class_engine_try_cast() -> BenchResult {
    let obj = RefCounted::new_gd().upcast::<Object>();

    bench_measure(|| obj.clone().try_cast::<RefCounted>())
}

/// Godot -> Rust `#[var]` read, driven from GDScript; see [`super::call_bench::class_user_refc_ptrcall()`] for the loop shape.
/// Reported time covers 100 accesses. ~2x a `#[func]` ptrcall per access, since property access goes through a generated getter.
#[bench(manual)]
fn class_user_property_get() -> BenchResult {
    let obj = BenchObj::new_gd().to_variant();
    let hammer = "hammer".to_string_name();

    let mut caller = RefCounted::new_gd();
    caller.set_script(&create_gdscript(
        "extends RefCounted\n\nfunc hammer(o: BenchObj) -> void:\n\tfor i in 100:\n\t\tvar _v = o.bench_int\n",
    ));

    bench_measure(|| caller.call(&hammer, std::slice::from_ref(&obj)))
}

/// Same as [`class_user_property_get()`], for the write direction.
#[bench(manual)]
fn class_user_property_set() -> BenchResult {
    let obj = BenchObj::new_gd().to_variant();
    let hammer = "hammer".to_string_name();

    let mut caller = RefCounted::new_gd();
    caller.set_script(&create_gdscript(
        "extends RefCounted\n\nfunc hammer(o: BenchObj) -> void:\n\tfor i in 100:\n\t\to.bench_int = i\n",
    ));

    bench_measure(|| caller.call(&hammer, std::slice::from_ref(&obj)))
}

#[bench]
fn class_singleton_access() -> Gd<Os> {
    Os::singleton()
}
