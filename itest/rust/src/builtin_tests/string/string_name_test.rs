/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::collections::HashSet;

use crate::framework::itest;
use godot::builtin::{GString, NodePath, StringName};

#[itest]
fn string_name_default() {
    let name = StringName::default();
    let back = GString::from(&name);

    assert_eq!(back, GString::new());
}

#[itest]
fn string_name_conversion() {
    // Note: StringName::from(&str) uses direct FFI constructor from Godot 4.2 onwards.

    let string = GString::from("some string");
    let name = StringName::from(&string);
    let back = GString::from(&name);

    assert_eq!(string, back);

    let second = StringName::from(string.clone());
    let back = GString::from(second);

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
fn string_name_ordering() {
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

#[itest]
#[cfg(since_api = "4.2")]
fn string_name_from_latin1_with_nul() {
    let cases: [(&[u8], &str); 3] = [
        (b"pure ASCII\t[~]\0", "pure ASCII\t[~]\0"),
        (b"\xB1\0", "±"),
        (b"Latin-1 \xA3 \xB1 text \xBE\0", "Latin-1 £ ± text ¾"),
    ];

    for (bytes, string) in cases.into_iter() {
        let a = StringName::from_latin1_with_nul(bytes);
        let b = StringName::from(string);

        println!();
        println!(
            "Arrays: a={:?}, b={:?}",
            a.to_string().as_bytes(),
            b.to_string().as_bytes()
        );
        println!("Hashes: a={:?}, b={:?}", a.hash(), b.hash());
        println!("Lengths: a={}, b={}", a.len(), b.len());
        assert_eq!(a, b);
    }
}
