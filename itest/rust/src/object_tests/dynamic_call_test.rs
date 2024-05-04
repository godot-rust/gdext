/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::builtin::meta::{CallError, FromGodot, ToGodot};
use godot::builtin::{StringName, Variant, Vector3};
use godot::engine::{Node, Node3D, Object};
use godot::obj::{InstanceId, NewAlloc};
use std::error::Error;

use crate::framework::{expect_panic, itest, runs_release};
use crate::object_tests::object_test::ObjPayload;

#[itest]
fn dynamic_call_no_args() {
    let mut node = Node3D::new_alloc().upcast::<Object>();

    let static_id = node.instance_id();
    let reflect_id_variant = node.call(StringName::from("get_instance_id"), &[]);

    let reflect_id = InstanceId::from_variant(&reflect_id_variant);

    assert_eq!(static_id, reflect_id);
    node.free();
}

#[itest]
fn dynamic_call_with_args() {
    let mut node = Node3D::new_alloc();

    let expected_pos = Vector3::new(2.5, 6.42, -1.11);

    let none = node.call(
        StringName::from("set_position"),
        &[expected_pos.to_variant()],
    );
    let actual_pos = node.call(StringName::from("get_position"), &[]);

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
        obj.call("take_1_int".into(), &[]);
    });

    // Use Result-based version.
    let call_error = obj
        .try_call("take_1_int".into(), &[])
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
        obj.call("take_1_int".into(), &[42.to_variant(), 43.to_variant()]);
    });

    // Use Result-based version.
    let call_error = obj
        .try_call("take_1_int".into(), &[42.to_variant(), 43.to_variant()])
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
        obj.call("take_1_int".into(), &["string".to_variant()]);
    });

    // Use Result-based version.
    let call_error = obj
        .try_call("take_1_int".into(), &["string".to_variant()])
        .expect_err("expected failed call");

    assert_eq!(call_error.class_name(), Some("Object"));
    assert_eq!(call_error.method_name(), "call");
    assert_eq!(
        call_error.to_string(),
        "godot-rust function call failed: Object::call(&\"take_1_int\", [va] \"string\")\
        \n  Source: ObjPayload::take_1_int()\
        \n    Reason: parameter #0 (i64) conversion\
        \n  Source: expected type Int, got String: \"string\""
    );

    obj.free();
}

#[itest]
fn dynamic_call_with_panic() {
    let mut obj = ObjPayload::new_alloc();

    let result = obj.try_call("do_panic".into(), &[]);
    let call_error = result.expect_err("panic should cause a call error");

    assert_eq!(call_error.class_name(), Some("Object"));
    assert_eq!(call_error.method_name(), "call");
    assert_eq!(
        call_error.to_string(),
        "godot-rust function call failed: Object::call(&\"do_panic\")\
        \n  Source: ObjPayload::do_panic()\
        \n    Reason: [panic]  do_panic exploded"
    );

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
        node.call("rpc_config".into(), &["some_method".to_variant()]);
    });

    // Use Result-based version.
    let call_error = node
        .try_call("rpc_config".into(), &["some_method".to_variant()])
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
        node.call(
            "rpc_config".into(),
            &["some_method".to_variant(), Variant::nil(), 123.to_variant()],
        );
    });

    // Use Result-based version.
    let call_error = node
        .try_call(
            "rpc_config".into(),
            &["some_method".to_variant(), Variant::nil(), 123.to_variant()],
        )
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
        node.call("set_name".into(), &[123.to_variant()]);
    });

    // Use Result-based version.
    let call_error = node
        .try_call("set_name".into(), &[123.to_variant()])
        .expect_err("expected failed call");

    // Note: currently no mention of Node::set_name(). Not sure if easily possible to add.
    assert_eq!(call_error.class_name(), Some("Object"));
    assert_eq!(call_error.method_name(), "call");
    assert_eq!(
        call_error.to_string(),
        "godot-rust function call failed: Object::call(&\"set_name\", [va] 123)\
        \n    Reason: parameter #1 conversion -- expected type String, got Int"
    );

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
