/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::builtin::meta::{CallError, FromGodot, ToGodot};
use godot::builtin::{StringName, Variant, Vector3};
use godot::engine::{Node3D, Object};
use godot::obj::{InstanceId, NewAlloc};
use std::error::Error;

use crate::framework::{expect_panic, itest};
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
        \n  Reason: function has 1 parameter, but received 0 arguments"
    );

    // Method where error originated (this is not repeated in all tests, the logic for chaining is the same).
    let source = call_error.source().expect("must have source CallError");
    assert_eq!(
        source.to_string(),
        "godot-rust function call failed: ObjPayload::take_1_int()\
        \n  Reason: function has 1 parameter, but received 0 arguments"
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
        "godot-rust function call failed: Object::call(&\"take_1_int\", varargs 42, 43)\
        \n  Source: ObjPayload::take_1_int()\
        \n  Reason: function has 1 parameter, but received 2 arguments"
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
        \n  Reason: Panic msg:  do_panic exploded"
    );

    obj.free();
}
