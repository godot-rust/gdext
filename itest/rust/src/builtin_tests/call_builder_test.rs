/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Tests for the unified call builder (`call_ex`) shared by `Gd`, `Variant` and `Callable`.

use godot::builtin::{Signal, Variant, varray};
use godot::meta::ToGodot;
use godot::obj::{Base, Gd, NewGd};
use godot::prelude::{GodotClass, godot_api};
use godot::task::{self, TaskHandle};

use crate::framework::{TestContext, itest};

#[derive(GodotClass)]
#[class(init, base = RefCounted)]
struct CallBuilderObj {
    base: Base<godot::classes::RefCounted>,
    value: i64,
}

#[godot_api]
impl CallBuilderObj {
    #[func]
    fn sum(&self, a: i64, b: i64) -> i64 {
        a + b
    }

    // Mutator used to observe deferred calls, which have no return value.
    #[func]
    fn set_value(&mut self, value: i64) {
        self.value = value;
    }
}

fn new_obj() -> Gd<CallBuilderObj> {
    CallBuilderObj::new_gd()
}

#[itest]
fn call_builder_gd_done() {
    let obj = new_obj();
    let result = obj
        .call_ex("sum")
        .args(&[2.to_variant(), 3.to_variant()])
        .done();
    assert_eq!(result, 5.to_variant());
}

#[itest]
fn call_builder_variant_matches_shorthand() {
    let obj = new_obj();
    let variant = obj.to_variant();
    let args = [2.to_variant(), 3.to_variant()];

    // `call_ex(...).args(...).done()` is equivalent to the bare `call(...)` shorthand.
    let via_builder = variant.call_ex("sum").args(&args).done();
    let via_shorthand = variant.call("sum", &args);
    assert_eq!(via_builder, via_shorthand);
    assert_eq!(via_builder, 5.to_variant());
}

#[itest]
fn call_builder_try_done_ok() {
    let obj = new_obj();
    let result = obj
        .call_ex("sum")
        .args(&[10.to_variant(), 20.to_variant()])
        .try_done();
    assert_eq!(result.unwrap(), 30.to_variant());
}

#[itest]
fn call_builder_try_done_err() {
    let obj = new_obj();
    let result = obj.call_ex("nonexistent_method").try_done();
    assert!(result.is_err());
}

#[itest]
fn call_builder_args_array() {
    let obj = new_obj();
    let args = varray![4, 5];

    let via_array = obj.call_ex("sum").args_array(&args).done();
    let via_slice = obj
        .call_ex("sum")
        .args(&[4.to_variant(), 5.to_variant()])
        .done();
    assert_eq!(via_array, via_slice);
    assert_eq!(via_array, 9.to_variant());
}

#[itest]
fn call_builder_callable_invoke() {
    let obj = new_obj();
    let callable = obj.callable("sum");

    let result = callable
        .call_ex()
        .args(&[6.to_variant(), 7.to_variant()])
        .done();
    assert_eq!(result, 13.to_variant());
}

#[itest]
fn call_builder_callable_try_done_always_ok() {
    // A `Callable` invocation can't surface call errors (Godot returns NIL), so `try_done()` reports `Ok` even on failure.
    let callable = new_obj().callable("nonexistent_method");

    let result = callable.call_ex().try_done();
    assert_eq!(result.unwrap(), Variant::nil());
}

// `deferred()` runs the call at idle time, not synchronously; the mutation is visible only after the next frame.
#[itest(async)]
fn call_builder_deferred_gd(ctx: &TestContext) -> TaskHandle {
    let obj = new_obj();

    obj.call_ex("set_value").args(&[99.to_variant()]).deferred();
    assert_eq!(obj.bind().value, 0, "deferred call ran synchronously");

    let frame = Signal::from_object_signal(&ctx.scene_tree.get_tree(), "process_frame");
    task::spawn(async move {
        frame.to_future::<()>().await;
        assert_eq!(obj.bind().value, 99);
    })
}

// Same as above, but invoking a `Callable` target instead of naming a method.
#[itest(async)]
fn call_builder_deferred_callable(ctx: &TestContext) -> TaskHandle {
    let obj = new_obj();
    let callable = obj.callable("set_value");

    callable.call_ex().args(&[123.to_variant()]).deferred();
    assert_eq!(obj.bind().value, 0, "deferred call ran synchronously");

    let frame = Signal::from_object_signal(&ctx.scene_tree.get_tree(), "process_frame");
    task::spawn(async move {
        frame.to_future::<()>().await;
        assert_eq!(obj.bind().value, 123);
    })
}

// A non-coroutine method resolves immediately: `to_future()` returns the value without an actual `await` suspension.
#[itest(async)]
fn call_builder_to_future_immediate() -> TaskHandle {
    let obj = new_obj();

    task::spawn(async move {
        let result = obj
            .call_ex("sum")
            .args(&[8.to_variant(), 9.to_variant()])
            .to_future()
            .await;
        assert_eq!(result, 17.to_variant());
    })
}

// A GDScript method using `await` returns a coroutine handle; `to_future()` must await its completion before resolving.
#[cfg(since_api = "4.3")]
#[itest(async)]
fn call_builder_to_future_coroutine(ctx: &crate::framework::TestContext) -> TaskHandle {
    use godot::classes::Node;
    use godot::obj::NewAlloc;

    use crate::framework::create_gdscript;

    let script = create_gdscript(
        r#"
extends Node

func compute(input: int) -> int:
    await get_tree().process_frame
    return input * 2
"#,
    );

    let mut node = Node::new_alloc();
    node.set_script(&script);

    let mut tree = ctx.scene_tree.clone();
    tree.add_child(&node);

    task::spawn(async move {
        let result = node
            .call_ex("compute")
            .args(&[21.to_variant()])
            .to_future()
            .await;
        assert_eq!(result, 42.to_variant());

        tree.remove_child(&node);
        node.free();
    })
}
