/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::collections::HashSet;

use crate::itest;
use godot::builtin::{GodotString, NodePath};

#[itest]
fn node_path_default() {
    let name = NodePath::default();
    let back = GodotString::from(&name);

    assert_eq!(back, GodotString::new());
}

#[itest]
fn node_path_conversion() {
    let string = GodotString::from("some string");
    let name = NodePath::from(&string);
    let back = GodotString::from(&name);

    assert_eq!(string, back);

    let second = NodePath::from(string.clone());
    let back = GodotString::from(second);

    assert_eq!(string, back);
}
#[itest]
fn node_path_equality() {
    let string = NodePath::from("some string");
    let second = NodePath::from("some string");
    let different = NodePath::from("some");

    assert_eq!(string, second);
    assert_ne!(string, different);
}

#[itest]
fn node_path_clone() {
    let first = NodePath::from("some string");
    #[allow(clippy::redundant_clone)]
    let cloned = first.clone();

    assert_eq!(first, cloned);
}

#[itest]
fn node_path_hash() {
    let set: HashSet<NodePath> = [
        "string_1",
        "SECOND string! :D",
        "emoji time: 😎",
        r#"got/!()%)=!"/]}¡[$½{¥¡}@£symbol characters"#,
        "some garbageTƉ馧쟻�韂󥢛ꮛ૎ཾ̶D@/8ݚ򹾴-䌗򤷨񄣷8",
    ]
    .into_iter()
    .map(NodePath::from)
    .collect();
    assert_eq!(set.len(), 5);
}
