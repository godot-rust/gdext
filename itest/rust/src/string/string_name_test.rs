/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::collections::HashSet;

use crate::itest;
use godot::builtin::{GodotString, NodePath, StringName};

#[itest]
fn string_name_default() {
    let name = StringName::default();
    let back = GodotString::from(&name);

    assert_eq!(back, GodotString::new());
}

#[itest]
fn string_name_conversion() {
    let string = GodotString::from("some string");
    let name = StringName::from(&string);
    let back = GodotString::from(&name);

    assert_eq!(string, back);

    let second = StringName::from(string.clone());
    let back = GodotString::from(second);

    assert_eq!(string, back);
}

#[itest]
fn string_name_node_path_conversion() {
    let string = StringName::from("some string");
    let name = NodePath::from(&string);
    let back = StringName::from(&name);

    assert_eq!(string, back);

    let second = NodePath::from(string.clone());
    let back = StringName::from(second);

    assert_eq!(string, back);
}

#[itest]
fn string_name_equality() {
    let string = StringName::from("some string");
    let second = StringName::from("some string");
    let different = StringName::from("some");

    assert_eq!(string, second);
    assert_ne!(string, different);
}

// TODO: add back in when ordering StringNames is fixed
#[itest(skip)]
fn string_ordering() {
    let _low = StringName::from("Alpha");
    let _high = StringName::from("Beta");
    /*
    assert!(low < high);
    assert!(low <= high);
    assert!(high > low);
    assert!(high >= low);
     */
}

#[itest]
fn string_name_clone() {
    let first = StringName::from("some string");
    #[allow(clippy::redundant_clone)]
    let cloned = first.clone();

    assert_eq!(first, cloned);
}

#[itest]
fn string_name_hash() {
    let set: HashSet<StringName> = [
        "string_1",
        "SECOND string! :D",
        "emoji time: 😎",
        r#"got/!()%)=!"/]}¡[$½{¥¡}@£symbol characters"#,
        "some garbageTƉ馧쟻�韂󥢛ꮛ૎ཾ̶D@/8ݚ򹾴-䌗򤷨񄣷8",
    ]
    .into_iter()
    .map(StringName::from)
    .collect();
    assert_eq!(set.len(), 5);
}

#[itest]
fn string_name_length() {
    let string = "hello!";
    let name = StringName::from(string);
    assert_eq!(name.len(), string.len());

    let empty = StringName::default();
    assert_eq!(empty.len(), 0);
}

#[itest]
fn string_name_is_empty() {
    let name = StringName::from("hello!");
    assert!(!name.is_empty());
    let empty = StringName::default();
    assert!(empty.is_empty());
}
