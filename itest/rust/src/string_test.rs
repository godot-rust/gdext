/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::itest;
use godot::builtin::{GodotString, StringName};

// TODO use tests from godot-rust/gdnative

#[itest]
fn string_default() {
    let string = GodotString::new();
    let back = String::from(&string);

    assert_eq!(back.as_str(), "");
}

#[itest]
fn string_conversion() {
    let string = String::from("some string");
    let second = GodotString::from(&string);
    let back = String::from(&second);

    assert_eq!(string, back);
}

#[itest]
fn string_equality() {
    let string = GodotString::from("some string");
    let second = GodotString::from("some string");
    let different = GodotString::from("some");

    assert_eq!(string, second);
    assert_ne!(string, different);
}

#[itest]
fn string_ordering() {
    let low = GodotString::from("Alpha");
    let high = GodotString::from("Beta");

    assert!(low < high);
    assert!(low <= high);
    assert!(high > low);
    assert!(high >= low);
}

#[itest]
fn string_clone() {
    let first = GodotString::from("some string");
    let cloned = first.clone();

    assert_eq!(first, cloned);
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[itest]
fn string_name_conversion() {
    let string = GodotString::from("some string");
    let name = StringName::from(&string);
    let back = GodotString::from(&name);

    assert_eq!(string, back);
}

#[itest]
fn string_name_default_construct() {
    let name = StringName::default();
    let back = GodotString::from(&name);

    assert_eq!(back, GodotString::new());
}

#[itest]
fn string_name_eq_hash() {
    // TODO
}

#[itest]
fn string_name_ord() {
    // TODO
}

#[itest]
fn string_name_clone() {
    // TODO
}
