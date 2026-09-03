/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Benchmarks are grouped by what they measure: `anchor` (plain Rust, as reference), `builtin` (values), `call` (method calls in both
//! directions), `callable` (`Callable` invocation), `color` (color conversions), `object` (instance access and lifetime), `variant`
//! (conversions). Each one tests a single thing, so a slowdown points to a specific place.

use godot::builtin::{GString, GodotStringExt};
use godot::classes::notify::ObjectNotification;
use godot::classes::{IRefCounted, RefCounted};
use godot::obj::Base;
use godot::register::{GodotClass, godot_api};

mod anchor_bench;
mod builtin_bench;
mod call_bench;
mod callable_bench;
mod color_bench;
mod object_bench;
mod variant_bench;

#[derive(GodotClass)]
#[class(init)]
pub(super) struct BenchObj {
    #[var]
    bench_int: i64,
}

#[godot_api]
impl BenchObj {
    #[func]
    fn noop_ref(&self) {}

    #[func]
    fn noop_mut(&mut self) {}

    #[func]
    fn echo(&self, text: GString) -> GString {
        text
    }

    #[func]
    fn echo_default(&self, #[opt(default = "some test string")] text: GString) -> GString {
        text
    }
}

/// Separate class, so that the base field and signal registration don't affect the benchmarks using [`BenchObj`].
#[derive(GodotClass)]
#[class(init, base = RefCounted)]
pub(super) struct SignalBenchObj {
    _base: Base<RefCounted>,
}

#[godot_api]
impl SignalBenchObj {
    #[signal]
    fn bench_signal();
}

/// Separate class, so that the virtual methods don't affect the benchmarks using [`BenchObj`].
#[derive(GodotClass)]
#[class(init, base = RefCounted)]
pub(super) struct VirtualBenchObj {
    _base: Base<RefCounted>,
}

/// `#[func(virtual)]` requires Godot 4.3.
#[cfg(since_api = "4.3")]
#[godot_api]
impl VirtualBenchObj {
    /// Forwarded to a GDScript override if one is attached; see [`call_bench::class_user_script_virtual()`].
    #[func(virtual)]
    fn bench_virtual(&self) -> i64 {
        0
    }
}

#[godot_api]
impl IRefCounted for VirtualBenchObj {
    fn on_notification(&mut self, _what: ObjectNotification) {}

    fn to_string(&self) -> GString {
        "VirtualBenchObj".to_gstring()
    }
}
