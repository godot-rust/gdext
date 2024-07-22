/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::builtin::Variant;
use godot::classes::{ClassDb, Node};
use godot::global;
use godot::obj::{Gd, NewAlloc};

use crate::framework::itest;
use crate::object_tests::object_test::{user_refc_instance, RefcPayload};

#[itest]
fn object_arg_owned() {
    with_objects(|manual, refc| {
        let db = ClassDb::singleton();
        let a = db.class_set_property(manual, "name".into(), Variant::from("hello"));
        let b = db.class_set_property(refc, "value".into(), Variant::from(-123));
        (a, b)
    });
}

#[itest]
fn object_arg_borrowed() {
    with_objects(|manual, refc| {
        let db = ClassDb::singleton();
        let a = db.class_set_property(&manual, "name".into(), Variant::from("hello"));
        let b = db.class_set_property(&refc, "value".into(), Variant::from(-123));
        (a, b)
    });
}

#[itest]
fn object_arg_option_owned() {
    with_objects(|manual, refc| {
        let db = ClassDb::singleton();
        let a = db.class_set_property(Some(manual), "name".into(), Variant::from("hello"));
        let b = db.class_set_property(Some(refc), "value".into(), Variant::from(-123));
        (a, b)
    });
}

#[itest]
fn object_arg_option_borrowed() {
    with_objects(|manual, refc| {
        let db = ClassDb::singleton();
        let a = db.class_set_property(Some(&manual), "name".into(), Variant::from("hello"));
        let b = db.class_set_property(Some(&refc), "value".into(), Variant::from(-123));
        (a, b)
    });
}

#[itest]
fn object_arg_option_none() {
    let manual: Option<Gd<Node>> = None;
    let refc: Option<Gd<RefcPayload>> = None;

    // Will emit errors but should not crash.
    let db = ClassDb::singleton();
    let error = db.class_set_property(manual, "name".into(), Variant::from("hello"));
    assert_eq!(error, global::Error::ERR_UNAVAILABLE);

    let error = db.class_set_property(refc, "value".into(), Variant::from(-123));
    assert_eq!(error, global::Error::ERR_UNAVAILABLE);
}

#[itest]
fn object_arg_null_arg() {
    // Will emit errors but should not crash.
    let db = ClassDb::singleton();
    let error = db.class_set_property(Gd::null_arg(), "name".into(), Variant::from("hello"));
    assert_eq!(error, global::Error::ERR_UNAVAILABLE);

    let error = db.class_set_property(Gd::null_arg(), "value".into(), Variant::from(-123));
    assert_eq!(error, global::Error::ERR_UNAVAILABLE);
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Helpers

fn with_objects<F>(f: F)
where
    F: FnOnce(Gd<Node>, Gd<RefcPayload>) -> (global::Error, global::Error),
{
    let manual = Node::new_alloc();
    let refc = user_refc_instance();

    let manual2 = manual.clone();
    let refc2 = refc.clone();

    let (a, b) = f(manual, refc);

    assert_eq!(a, global::Error::OK);
    assert_eq!(b, global::Error::OK);
    assert_eq!(manual2.get_name(), "hello".into());
    assert_eq!(refc2.bind().value, -123);

    manual2.free();
}
