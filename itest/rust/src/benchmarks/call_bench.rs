/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::builtin::GodotStringExt;
use godot::classes::RefCounted;
use godot::classes::notify::ObjectNotification;
use godot::meta::ToGodot;
use godot::obj::NewGd;

use super::{BenchObj, SignalBenchObj, VirtualBenchObj};
use crate::framework::{BenchResult, bench, bench_measure, create_gdscript};

/// Godot -> Rust call into a `#[func]` method, through the trampoline that keeps the storage alive for the call's duration.
#[bench(manual)]
fn class_user_refc_call_ref() -> BenchResult {
    let obj = BenchObj::new_gd();
    let callable = obj.callable("noop_ref");

    bench_measure(|| callable.call(&[]))
}

/// Same as [`class_user_refc_call_ref()`], but the receiver additionally requires an exclusive bind.
#[bench(manual)]
fn class_user_refc_call_mut() -> BenchResult {
    let obj = BenchObj::new_gd();
    let callable = obj.callable("noop_mut");

    bench_measure(|| callable.call(&[]))
}

/// Same as [`class_user_refc_call_ref()`], but marshalling one argument and one return value.
#[bench(manual)]
fn class_user_refc_call_args() -> BenchResult {
    let obj = BenchObj::new_gd();
    let callable = obj.callable("echo");
    let args = ["some test string".to_variant()];

    bench_measure(|| callable.call(&args))
}

/// Godot -> Rust call into a virtual method, through the panic-handling chain. `Object::to_string()` is not public, so this also
/// includes the `GString` -> `String` conversion measured by `builtin_string_to_rust`.
#[bench(manual)]
fn class_user_virtual_to_string() -> BenchResult {
    let obj = VirtualBenchObj::new_gd();

    bench_measure(|| obj.to_string())
}

/// Same as [`class_user_virtual_to_string()`], but the virtual method itself does no work.
#[bench(manual)]
fn class_user_virtual_notify() -> BenchResult {
    let mut obj = VirtualBenchObj::new_gd();

    bench_measure(|| {
        obj.notify(ObjectNotification::Unknown(1_000_000)); // Neutral: no lifecycle meaning.
        true
    })
}

/// Rust -> Godot varcall into an engine method, through `Signature::out_class_varcall` (dynamic dispatch, `Variant` marshalling).
#[bench(manual)]
fn class_engine_out_varcall() -> BenchResult {
    let mut obj = RefCounted::new_gd();
    let method = "get_reference_count".to_string_name();

    bench_measure(|| obj.call(&method, &[]))
}

/// Same as [`class_engine_out_varcall()`], but with one explicit argument, so the argument tuple is non-empty.
#[bench(manual)]
fn class_engine_out_varcall_arg() -> BenchResult {
    let mut obj = RefCounted::new_gd();
    let method = "has_meta".to_string_name();
    let args = ["some_meta".to_variant()];

    bench_measure(|| obj.call(&method, &args))
}

/// Same engine method as [`class_engine_out_varcall()`], but statically typed, through `Signature::out_class_ptrcall`.
// ~4x faster than the varcall, which is the cost of dynamic dispatch plus `Variant` marshalling.
#[bench(manual)]
fn class_engine_out_ptrcall() -> BenchResult {
    let obj = RefCounted::new_gd();

    bench_measure(|| obj.get_reference_count())
}

/// Same as [`class_engine_out_ptrcall()`], but with one argument, so `GodotFfi::to_arg_ptr()` runs per call.
#[bench(manual)]
fn class_engine_out_ptrcall_arg() -> BenchResult {
    let obj = RefCounted::new_gd();
    let meta = "some_meta".to_string_name();

    bench_measure(|| obj.has_meta(&meta))
}

/// Godot -> Rust `#[func]` call through the ptrcall glue: GDScript emits ptrcall, not varcall, for a statically typed receiver.
///
/// Each measured iteration is one out-varcall into GDScript plus 100 inbound ptrcalls, so the latter dominate. Divide the reported
/// time by 100 to compare against the per-call benchmarks above.
#[bench(manual)]
fn class_user_refc_ptrcall() -> BenchResult {
    let obj = BenchObj::new_gd().to_variant();
    let hammer = "hammer".to_string_name();

    let mut caller = RefCounted::new_gd();
    caller.set_script(&create_gdscript(
        "extends RefCounted\n\nfunc hammer(o: BenchObj) -> void:\n\tfor i in 100:\n\t\to.noop_ref()\n",
    ));

    bench_measure(|| caller.call(&hammer, std::slice::from_ref(&obj)))
}

/// Same as [`class_user_refc_ptrcall()`], but marshalling one argument and one return value.
#[bench(manual)]
fn class_user_refc_ptrcall_args() -> BenchResult {
    let obj = BenchObj::new_gd().to_variant();
    let hammer = "hammer".to_string_name();

    let mut caller = RefCounted::new_gd();
    caller.set_script(&create_gdscript(
        "extends RefCounted\n\nfunc hammer(o: BenchObj) -> void:\n\tfor i in 100:\n\t\to.echo(\"some test string\")\n",
    ));

    bench_measure(|| caller.call(&hammer, std::slice::from_ref(&obj)))
}

/// Rust -> Godot call into a GDScript override of a Rust virtual, through `Signature::out_script_virtual_call`.
#[cfg(since_api = "4.3")]
#[bench(manual)]
fn class_user_script_virtual() -> BenchResult {
    let mut obj = VirtualBenchObj::new_gd();
    obj.set_script(&create_gdscript(
        "extends VirtualBenchObj\n\nfunc _bench_virtual() -> int:\n\treturn 0\n",
    ));

    bench_measure(|| obj.bind().bench_virtual())
}

#[bench(manual)]
fn class_user_signal_emit() -> BenchResult {
    let obj = SignalBenchObj::new_gd();
    obj.signals().bench_signal().connect(|| {});

    bench_measure(|| {
        obj.signals().bench_signal().emit();
        true
    })
}
