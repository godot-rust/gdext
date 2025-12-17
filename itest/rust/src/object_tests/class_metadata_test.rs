/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::builtin::GString;
use godot::classes::{IRefCounted, Node, RefCounted};
use godot::obj::{Base, ClassMetadata};
use godot::register::{godot_api, GodotClass};

use crate::framework::itest;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Test classes

#[derive(GodotClass)]
#[class(no_init, base=RefCounted)]
struct MetadataTestClass {
    base: Base<RefCounted>,

    #[var]
    test_property: i32,

    #[export]
    exported_property: GString,
}

#[godot_api]
impl MetadataTestClass {
    #[func]
    fn test_function(&self) -> i32 {
        42
    }
}

#[godot_api]
impl IRefCounted for MetadataTestClass {}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Tests for user-defined class properties

#[itest]
fn user_class_property_metadata() {
    assert!(MetadataTestClass::__class_has_property("test_property"));
    assert!(MetadataTestClass::__class_has_property("exported_property"));

    assert!(!MetadataTestClass::__class_has_property(
        "nonexistent_property"
    ));
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Tests for user-defined class functions

#[itest]
fn user_class_function_metadata() {
    assert!(MetadataTestClass::__class_has_function("test_function"));

    assert!(!MetadataTestClass::__class_has_function(
        "nonexistent_function"
    ));
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Tests for engine class properties

#[itest]
fn engine_class_property_metadata() {
    assert!(Node::__class_has_property("name"));
    assert!(Node::__class_has_property("process_mode"));

    assert!(!Node::__class_has_property("nonexistent_property"));
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Tests for engine class functions

#[itest]
fn engine_class_function_metadata() {
    assert!(Node::__class_has_function("add_child"));
    assert!(Node::__class_has_function("get_parent"));

    assert!(!Node::__class_has_function("nonexistent_function"));
}
