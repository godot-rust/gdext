/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::error::Error;
use std::sync::{Arc, Mutex};

use godot::builtin::{vslice, Variant, Vector3};
use godot::classes::{Node, Node3D, Object};
use godot::init::GdextBuild;
use godot::meta::error::CallError;
use godot::meta::{FromGodot, ToGodot};
use godot::obj::{InstanceId, NewAlloc};

use crate::framework::{expect_panic, itest, runs_release};
use crate::object_tests::object_test::ObjPayload;

#[itest]
fn dynamic_call_no_args() {
    let mut node = Node3D::new_alloc().upcast::<Object>();

    let static_id = node.instance_id();
    let reflect_id_variant = node.call("get_instance_id", &[]);

    let reflect_id = InstanceId::from_variant(&reflect_id_variant);

    assert_eq!(static_id, reflect_id);
    node.free();
}

#[itest]
fn dynamic_call_with_args() {
    let mut node = Node3D::new_alloc();

    let expected_pos = Vector3::new(2.5, 6.42, -1.11);

    let none = node.call("set_position", vslice![expected_pos]);
    let actual_pos = node.call("get_position", &[]);

    assert_eq!(none, Variant::nil());
    assert_eq!(actual_pos, expected_pos.to_variant());
    node.free();
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Erroneous dynamic calls to #[func]

#[itest]
fn dynamic_call_with_too_few_args() {
    let mut obj = ObjPayload::new_alloc();

    // Use panicking version.
    expect_panic("call with too few arguments", || {
        obj.call("take_1_int", &[]);
    });

    // Use Result-based version.
    let call_error = obj
        .try_call("take_1_int", &[])
        .expect_err("expected failed call");

    // User-facing method to which error was propagated.
    assert_eq!(call_error.class_name(), Some("Object"));
    assert_eq!(call_error.method_name(), "call");
    assert_eq!(
        call_error.to_string(),
        "godot-rust function call failed: Object::call(&\"take_1_int\")\
        \n  Source: ObjPayload::take_1_int()\
        \n    Reason: function has 1 parameter, but received 0 arguments"
    );

    // Method where error originated (this is not repeated in all tests, the logic for chaining is the same).
    let source = call_error.source().expect("must have source CallError");
    assert_eq!(
        source.to_string(),
        "godot-rust function call failed: ObjPayload::take_1_int()\
        \n    Reason: function has 1 parameter, but received 0 arguments"
    );

    let source = source
        .downcast_ref::<CallError>()
        .expect("source must be CallError");
    assert_eq!(source.class_name(), Some("ObjPayload"));
    assert_eq!(source.method_name(), "take_1_int");

    obj.free();
}

#[itest]
fn dynamic_call_with_too_many_args() {
    let mut obj = ObjPayload::new_alloc();

    // Use panicking version.
    expect_panic("call with too many arguments", || {
        obj.call("take_1_int", vslice![42, 43]);
    });

    // Use Result-based version.
    let call_error = obj
        .try_call("take_1_int", vslice![42, 43])
        .expect_err("expected failed call");

    assert_eq!(call_error.class_name(), Some("Object"));
    assert_eq!(call_error.method_name(), "call");
    assert_eq!(
        call_error.to_string(),
        "godot-rust function call failed: Object::call(&\"take_1_int\", [va] 42, 43)\
        \n  Source: ObjPayload::take_1_int()\
        \n    Reason: function has 1 parameter, but received 2 arguments"
    );

    obj.free();
}

#[itest]
fn dynamic_call_parameter_mismatch() {
    let mut obj = ObjPayload::new_alloc();

    // Use panicking version.
    expect_panic("call with wrong argument type", || {
        obj.call("take_1_int", vslice!["string"]);
    });

    // Use Result-based version.
    let call_error = obj
        .try_call("take_1_int", vslice!["string"])
        .expect_err("expected failed call");

    assert_eq!(call_error.class_name(), Some("Object"));
    assert_eq!(call_error.method_name(), "call");
    assert_eq!(
        call_error.to_string(),
        "godot-rust function call failed: Object::call(&\"take_1_int\", [va] \"string\")\
        \n  Source: ObjPayload::take_1_int()\
        \n    Reason: parameter #0 (i64) conversion\
        \n  Source: cannot convert from STRING to INT: \"string\""
    );

    obj.free();
}

// There seems to be a weird bug where running *only* this test with #[itest (focus)] causes panic, which then causes a
// follow-up failure of Gd::bind_mut(), preventing benchmarks from being run. Doesn't happen with #[itest], when running all.
#[itest]
fn dynamic_call_with_panic() {
    let panic_message = Arc::new(Mutex::new(None));
    let panic_message_clone = panic_message.clone();

    std::panic::set_hook(Box::new(move |panic_info| {
        let error_message = godot::private::format_panic_message(panic_info);
        *panic_message_clone.lock().unwrap() =
            Some((error_message, godot::private::get_gdext_panic_context()));
    }));

    let mut obj = ObjPayload::new_alloc();

    let result = obj.try_call("do_panic", &[]);
    let call_error = result.expect_err("panic should cause a call error");

    assert_eq!(call_error.class_name(), Some("Object"));
    assert_eq!(call_error.method_name(), "call");

    let expected_error_message = "godot-rust function call failed: Object::call(&\"do_panic\")\
        \n  Source: ObjPayload::do_panic()\
        \n    Reason: function panicked: do_panic exploded 💥"
        .to_string();

    assert_eq!(call_error.to_string(), expected_error_message);

    let (panic_message, error_context) = panic_message
        .lock()
        .unwrap()
        .clone()
        .expect("panic message/context absent");

    let mut path = "itest/rust/src/object_tests/object_test.rs".to_string();
    if cfg!(target_os = "windows") {
        path = path.replace('/', "\\")
    }

    // Obtain line number dynamically -- avoids tedious maintenance on code reorganization.
    let line = ObjPayload::get_panic_line();
    let context = error_context
        .map(|context| format!("\n  Context: {context}"))
        .unwrap_or_default();

    // In Debug, there is a context -> message is multi-line -> '\n' is inserted after [panic ...].
    // In Release, simpler message -> single line -> no '\n'.
    let expected_panic_message = if cfg!(debug_assertions) {
        format!("[panic {path}:{line}]\n  do_panic exploded 💥{context}")
    } else {
        format!("[panic {path}:{line}]  do_panic exploded 💥")
    };

    assert_eq!(panic_message, expected_panic_message);

    obj.free();
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Erroneous dynamic calls to engine APIs

#[itest]
fn dynamic_call_with_too_few_args_engine() {
    // Disabled in release (parameter count is unchecked by engine).
    // Before 4.2, the Godot check had a bug: https://github.com/godotengine/godot/pull/80844.
    if runs_release() || cfg!(before_api = "4.2") {
        return;
    }

    let mut node = Node::new_alloc();

    // Use panicking version.
    expect_panic("call with too few arguments", || {
        node.call("rpc_config", vslice!["some_method"]);
    });

    // Use Result-based version.
    let call_error = node
        .try_call("rpc_config", vslice!["some_method"])
        .expect_err("expected failed call");

    assert_eq!(call_error.class_name(), Some("Object"));
    assert_eq!(call_error.method_name(), "call");
    assert_eq!(
        call_error.to_string(),
        "godot-rust function call failed: Object::call(&\"rpc_config\", [va] \"some_method\")\
        \n    Reason: function has 2 parameters, but received 1 argument"
    );

    node.free();
}

#[itest]
fn dynamic_call_with_too_many_args_engine() {
    // Disabled in release (parameter count is unchecked by engine).
    // Before 4.2, the Godot check had a bug: https://github.com/godotengine/godot/pull/80844.
    if runs_release() || cfg!(before_api = "4.2") {
        return;
    }

    let mut node = Node::new_alloc();

    // Use panicking version.
    expect_panic("call with too many arguments", || {
        node.call("rpc_config", vslice!["some_method", Variant::nil(), 123]);
    });

    // Use Result-based version.
    let call_error = node
        .try_call("rpc_config", vslice!["some_method", Variant::nil(), 123])
        .expect_err("expected failed call");

    assert_eq!(call_error.class_name(), Some("Object"));
    assert_eq!(call_error.method_name(), "call");
    assert_eq!(
        call_error.to_string(),
        "godot-rust function call failed: Object::call(&\"rpc_config\", [va] \"some_method\", null, 123)\
        \n    Reason: function has 2 parameters, but received 3 arguments"
    );

    node.free();
}

#[itest]
fn dynamic_call_parameter_mismatch_engine() {
    // Disabled in release (parameter types are unchecked by engine).
    if runs_release() {
        return;
    }

    let mut node = Node::new_alloc();

    // Use panicking version.
    expect_panic("call with wrong argument type", || {
        node.call("set_name", vslice![123]);
    });

    // Use Result-based version.
    let call_error = node
        .try_call("set_name", vslice![123])
        .expect_err("expected failed call");

    // Node::set_name() changed to accept StringName, in https://github.com/godotengine/godot/pull/76560.
    // Needs to check the runtime version rather than API version, because reflection calls always latest method (no compatibility method).
    let target_type = if GdextBuild::before_api("4.5") {
        "STRING"
    } else {
        "STRING_NAME"
    };
    let expected_error = format!(
        "godot-rust function call failed: Object::call(&\"set_name\", [va] 123)\
        \n    Reason: parameter #1 -- cannot convert from INT to {target_type}"
    );

    // Note: currently no mention of Node::set_name(). Not sure if easily possible to add.
    assert_eq!(call_error.class_name(), Some("Object"));
    assert_eq!(call_error.method_name(), "call");
    assert_eq!(call_error.to_string(), expected_error);

    node.free();
}

#[itest(skip)]
fn dynamic_call_return_mismatch() {
    // Cannot easily test this, as both calls to #[func] and Godot APIs are either strongly typed and correct (ensured by codegen),
    // or they return Variant, which then fails on user side only.

    // Even GDScript -> Rust calls cannot really use this. Given this GDScript code:
    //   var obj = ObjPayload.new()
    // 	 var result: String = obj.take_1_int(20)
    //
    // The parser will fail since it knows the signature of take_1_int(). And if we enforce `: Variant` type hints, it will just
    // cause a runtime error, but that's entirely handled in GDScript.
}
