/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// Needed for Clippy to accept #[cfg(all())]
#![allow(clippy::non_minimal_cfg)]

use godot::classes::GDScript;
use godot::global::godot_str;
use godot::prelude::*;
use godot::task::{self, TaskHandle};

use crate::framework::{TestContext, create_gdscript, itest, suppress_godot_print};

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Synchronous virtual functions

#[derive(GodotClass)]
#[class(init)]
struct VirtualScriptCalls {
    _base: Base<RefCounted>,
}

#[godot_api]
impl VirtualScriptCalls {
    #[func(virtual)]
    fn greet_lang(&self, i: i32) -> GString {
        godot_str!("Rust#{i}")
    }

    #[func(virtual, rename = greet_lang2)]
    fn gl2(&self, s: GString) -> GString {
        godot_str!("{s} Rust")
    }

    #[func(virtual, gd_self)]
    fn greet_lang3(_this: Gd<Self>, s: GString) -> GString {
        godot_str!("{s} Rust")
    }

    // Unlike `virtual`, registered under the plain name `speak` as a normal method, so Godot can always call it (Rust default when no
    // script, override otherwise).
    #[func(virtual_pub)]
    fn speak(&self, i: i32) -> GString {
        godot_str!("Rust#{i}")
    }

    #[func(virtual)]
    fn set_thing(&mut self, _input: Variant) {
        panic!("set_thing() must be overridden")
    }

    #[func(virtual)]
    fn get_thing(&self) -> Variant {
        panic!("get_thing() must be overridden")
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Tests

#[itest]
fn func_virtual() {
    // Without script: "Rust".
    let mut object = VirtualScriptCalls::new_gd();
    assert_eq!(object.bind().greet_lang(72), GString::from("Rust#72"));

    // With script: "GDScript".
    object.set_script(&make_script());
    assert_eq!(object.bind().greet_lang(72), GString::from("GDScript#72"));

    // Dynamic call: "GDScript".
    let result = object.call("_greet_lang", vslice![72]);
    assert_eq!(result, "GDScript#72".to_variant());
}

#[itest]
fn func_virtual_renamed() {
    // Without script: "Rust".
    let mut object = VirtualScriptCalls::new_gd();
    assert_eq!(
        object.bind().gl2("Hello".into()),
        GString::from("Hello Rust")
    );

    // With script: "GDScript".
    object.set_script(&make_script());
    assert_eq!(
        object.bind().gl2("Hello".into()),
        GString::from("Hello GDScript")
    );

    // Dynamic call: "GDScript".
    let result = object.call("greet_lang2", vslice!["Hello"]);
    assert_eq!(result, "Hello GDScript".to_variant());
}

#[itest]
fn func_virtual_gd_self() {
    // Without script: "Rust".
    let mut object = VirtualScriptCalls::new_gd();
    assert_eq!(
        VirtualScriptCalls::greet_lang3(object.clone(), "Hoi".into()),
        GString::from("Hoi Rust")
    );

    // With script: "GDScript".
    object.set_script(&make_script());
    assert_eq!(
        VirtualScriptCalls::greet_lang3(object.clone(), "Hoi".into()),
        GString::from("Hoi GDScript")
    );

    // Dynamic call: "GDScript".
    let result = object.call("_greet_lang3", vslice!["Hoi"]);
    assert_eq!(result, "Hoi GDScript".to_variant());
}

#[itest]
fn func_virtual_pub() {
    let mut object = VirtualScriptCalls::new_gd();

    // Key difference vs #[func(virtual)]: callable from Godot by plain name even without a script -> Rust default.
    assert_eq!(object.call("speak", vslice![72]), "Rust#72".to_variant());

    // Rust-side dispatch also hits the default.
    assert_eq!(object.bind().speak(72), GString::from("Rust#72"));

    // With script override: both Godot and Rust dispatch to GDScript.
    object.set_script(&make_script());
    assert_eq!(
        object.call("speak", vslice![72]),
        "GDScript#72".to_variant()
    );
    assert_eq!(object.bind().speak(72), GString::from("GDScript#72"));
}

// A `super` call in the override re-enters the dispatcher; without detection this recurses until the stack overflows.
#[itest]
fn func_virtual_pub_super_call_panics() {
    let mut object = VirtualScriptCalls::new_gd();
    object.set_script(&create_gdscript(
        r#"
extends VirtualScriptCalls

@warning_ignore("native_method_override")
func speak(i: int) -> String:
    return super.speak(i)
"#,
    ));

    // The guard panics across the FFI boundary, which gdext turns into a Godot error -- instead of recursing until the stack overflows.
    // The failed `super` call yields nil, which GDScript coerces to "" for the declared `-> String` return type.
    let result = suppress_godot_print(|| object.call("speak", vslice![72]));
    assert_eq!(result, "".to_variant());
}

#[itest]
fn func_virtual_stateful() {
    let mut object = VirtualScriptCalls::new_gd();
    object.set_script(&make_script());

    let variant = Vector3i::new(1, 2, 2).to_variant();
    object.bind_mut().set_thing(variant.clone());

    let retrieved = object.bind().get_thing();
    assert_eq!(retrieved, variant);
}

fn make_script() -> Gd<GDScript> {
    let code = r#"
extends VirtualScriptCalls

var thing

func _greet_lang(i: int) -> String:
    return str("GDScript#", i)
    
func greet_lang2(s: String) -> String:
    return str(s, " GDScript")

func _greet_lang3(s: String) -> String:
    return str(s, " GDScript")

# `virtual_pub` registers `speak` as a normal method, so overriding it warns NATIVE_METHOD_OVERRIDE (fatal here, warnings-as-errors). The
# override is still dispatched via gdext, so the warning is benign -- silence it.
@warning_ignore("native_method_override")
func speak(i: int) -> String:
    return str("GDScript#", i)

func _set_thing(anything):
    thing = anything

func _get_thing():
    return thing
"#;

    let script = create_gdscript(code);

    let methods = script
        .get_script_method_list()
        .iter_shared()
        .map(|dict| dict.get("name").unwrap())
        .collect::<VarArray>();

    // Ensure script has been parsed + compiled correctly.
    assert_eq!(script.get_instance_base_type(), "VirtualScriptCalls");
    assert_eq!(
        methods,
        varray![
            "_greet_lang",
            "greet_lang2",
            "_greet_lang3",
            "speak",
            "_set_thing",
            "_get_thing"
        ]
    );

    script
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Asynchronous virtual functions

// Do NOT merge this class with the RefCounted-based `VirtualScriptCalls` above. The coroutine test relies on `Base<Node>` so the GDScript
// override can `await get_tree().process_frame` -- the canonical scenario for async virtuals -- which is unavailable on `RefCounted`.
#[derive(GodotClass)]
#[class(init, base = Node)]
struct AsyncVirtualNode {
    base: Base<Node>,
}

#[godot_api]
impl AsyncVirtualNode {
    // `gd_self` is the recommended receiver for async virtuals: it avoids holding a `bind()` guard across the `.await`.
    #[func(virtual, gd_self)]
    async fn compute(_this: Gd<Self>, input: i64) -> i64 {
        // Synchronous Rust default, used when no script overrides `_compute`.
        input * 10
    }
}

/// Script overriding `_compute` with an `await`, so the call returns a `GDScriptFunctionState` first.
fn make_coroutine_script() -> Gd<GDScript> {
    create_gdscript(
        r#"
extends AsyncVirtualNode

func _compute(input: int) -> int:
    await get_tree().process_frame
    return input * 2
"#,
    )
}

/// Script overriding `_compute` synchronously (no `await`), returning the value directly.
fn make_sync_async_script() -> Gd<GDScript> {
    create_gdscript(
        r#"
extends AsyncVirtualNode

func _compute(input: int) -> int:
    return input * 2
"#,
    )
}

/// Attaches `script` (if any), awaits `compute(input)` and asserts the result, then cleans up.
fn run_async_compute(
    ctx: &TestContext,
    script: Option<Gd<GDScript>>,
    input: i64,
    expected: i64,
) -> TaskHandle {
    let mut node = AsyncVirtualNode::new_alloc();
    if let Some(script) = script {
        node.set_script(&script);
    }

    let mut tree = ctx.scene_tree.clone();
    tree.add_child(&node);

    task::spawn(async move {
        let result = AsyncVirtualNode::compute(node.clone(), input).await;
        assert_eq!(result, expected);

        tree.remove_child(&node);
        node.free();
    })
}

// No script attached -> the synchronous Rust default runs, but is still awaited through the async API.
#[itest(async)]
fn func_async_virtual_rust_sync(ctx: &TestContext) -> TaskHandle {
    run_async_compute(ctx, None, 5, 50)
}

// GDScript override without `await`: the call returns the value directly (no coroutine).
#[itest(async)]
fn func_async_virtual_gdscript_sync(ctx: &TestContext) -> TaskHandle {
    run_async_compute(ctx, Some(make_sync_async_script()), 21, 42)
}

// GDScript override with `await`: the call returns a coroutine handle, whose `completed` signal carries the eventual result.
#[itest(async)]
fn func_async_virtual_gdscript_coroutine(ctx: &TestContext) -> TaskHandle {
    run_async_compute(ctx, Some(make_coroutine_script()), 21, 42)
}
