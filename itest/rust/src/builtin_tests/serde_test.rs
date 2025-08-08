/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::builtin::{array, Array, Color, ColorHsv, GString, NodePath, StringName, Vector2i};
use serde::{Deserialize, Serialize};

use crate::framework::itest;

fn serde_roundtrip<T>(value: &T, expected_json: &str)
where
    T: for<'a> Deserialize<'a> + Serialize + PartialEq + std::fmt::Debug,
{
    let json: String = serde_json::to_string(value).unwrap();
    let back: T = serde_json::from_str(json.as_str()).unwrap();

    assert_eq!(back, *value, "serde round-trip changes value");
    assert_eq!(
        json, expected_json,
        "value does not conform to expected JSON"
    );
}

#[itest]
fn serde_gstring() {
    let value = GString::from("hello world");

    let expected_json = "\"hello world\"";

    serde_roundtrip(&value, expected_json);
}

#[itest]
fn serde_node_path() {
    let value = NodePath::from("res://icon.png");
    let expected_json = "\"res://icon.png\"";

    serde_roundtrip(&value, expected_json);
}

#[itest]
fn serde_string_name() {
    let value = StringName::from("hello world");
    let expected_json = "\"hello world\"";

    serde_roundtrip(&value, expected_json);
}

#[itest]
fn serde_array_rust_native_type() {
    let value: Array<i32> = array![1, 2, 3, 4, 5, 6];

    let expected_json = r#"[1,2,3,4,5,6]"#;

    serde_roundtrip(&value, expected_json)
}

#[itest]
fn serde_array_godot_builtin_type() {
    let value: Array<GString> = array!["Godot", "Rust", "Rocks"];

    let expected_json = r#"["Godot","Rust","Rocks"]"#;

    serde_roundtrip(&value, expected_json)
}

#[itest]
fn serde_array_godot_type() {
    let value: Array<Vector2i> = array![
        Vector2i::new(1, 1),
        Vector2i::new(2, 2),
        Vector2i::new(3, 3)
    ];

    let expected_json = r#"[{"x":1,"y":1},{"x":2,"y":2},{"x":3,"y":3}]"#;

    serde_roundtrip(&value, expected_json)
}

#[itest]
fn color_serde() {
    let color = Color::default();
    let expected_json = r#"{"r":0.0,"g":0.0,"b":0.0,"a":1.0}"#;
    serde_roundtrip(&color, expected_json);
}

#[itest]
fn color_hsv_serde() {
    let color = ColorHsv::default();
    let expected_json = r#"{"h":0.0,"s":0.0,"v":0.0,"a":1.0}"#;
    serde_roundtrip(&color, expected_json);
}
