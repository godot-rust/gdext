/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::builtin::GString;
use godot::classes::INode;
use godot::meta::ToGodot;
use godot::obj::NewAlloc;
use godot::register::{godot_api, GodotClass};
use godot::test::itest;

// Test 1: Valid override of engine class property.
#[derive(GodotClass)]
#[class(init, base=Node)]
struct ValidOverride {
    #[var(override)]
    name: GString,
}

#[godot_api]
impl INode for ValidOverride {}

#[itest]
fn var_override_valid() {
    let mut obj = ValidOverride::new_alloc();

    // Should be able to set and get the overridden property.
    let test_value = "test_name".to_variant();
    obj.set("name", &test_value);
    let name: GString = obj.get("name").to();
    assert_eq!(name, GString::from("test_name"));

    obj.free();
}

// Test 2: Property without override that doesn't conflict.
#[derive(GodotClass)]
#[class(init, base=Node)]
struct NoConflict {
    #[var]
    my_custom_property: i32,
}

#[godot_api]
impl INode for NoConflict {}

#[itest]
fn var_no_conflict() {
    let mut obj = NoConflict::new_alloc();

    // Should work fine - no conflict with base class.
    let value = 42_i32.to_variant();
    obj.set("my_custom_property", &value);
    let result: i32 = obj.get("my_custom_property").to();
    assert_eq!(result, 42_i32);

    obj.free();
}
