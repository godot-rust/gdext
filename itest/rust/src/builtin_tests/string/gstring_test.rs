/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::collections::HashSet;

use crate::framework::itest;
use godot::builtin::GString;

// TODO use tests from godot-rust/gdnative

#[itest]
fn string_default() {
    let string = GString::new();
    let back = String::from(&string);

    assert_eq!(back.as_str(), "");
}

#[itest]
fn string_conversion() {
    let string = String::from("some string");
    let second = GString::from(&string);
    let back = String::from(&second);

    assert_eq!(string, back);

    let second = GString::from(string.clone());
    let back = String::from(second);

    assert_eq!(string, back);
}

#[itest]
fn string_equality() {
    let string = GString::from("some string");
    let second = GString::from("some string");
    let different = GString::from("some");

    assert_eq!(string, second);
    assert_ne!(string, different);
}

#[itest]
fn string_ordering() {
    let low = GString::from("Alpha");
    let high = GString::from("Beta");

    assert!(low < high);
    assert!(low <= high);
    assert!(high > low);
    assert!(high >= low);
}

#[itest]
fn string_clone() {
    let first = GString::from("some string");
    #[allow(clippy::redundant_clone)]
    let cloned = first.clone();

    assert_eq!(first, cloned);
}

#[itest]
fn string_chars() {
    // Empty tests regression from #228: Null pointer passed to slice::from_raw_parts().
    let string = GString::new();
    let empty_char_slice: &[char] = &[];
    assert_eq!(string.chars(), empty_char_slice);
    assert_eq!(string, GString::from(empty_char_slice));

    let string = String::from("some_string");
    let string_chars: Vec<char> = string.chars().collect();
    let gstring = GString::from(string);

    assert_eq!(string_chars, gstring.chars().to_vec());
    assert_eq!(gstring, GString::from(string_chars.as_slice()));
}

#[itest]
fn string_hash() {
    let set: HashSet<GString> = [
        "string_1",
        "SECOND string! :D",
        "emoji time: 😎",
        r#"got/!()%)=!"/]}¡[$½{¥¡}@£symbol characters"#,
        "some garbageTƉ馧쟻�韂󥢛ꮛ૎ཾ̶D@/8ݚ򹾴-䌗򤷨񄣷8",
    ]
    .into_iter()
    .map(GString::from)
    .collect();
    assert_eq!(set.len(), 5);
}

#[itest]
fn string_with_null() {
    // Godot always ignores bytes after a null byte.
    let cases: &[(&str, &str)] = &[
        (
            "some random string",
            "some random string\0 with a null byte",
        ),
        ("", "\0"),
    ];

    for (left, right) in cases.iter() {
        let left = GString::from(*left);
        let right = GString::from(*right);

        assert_eq!(left, right);
    }
}
