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
fn empty_string_chars() {
    // Tests regression from #228: Null pointer passed to slice::from_raw_parts
    let s = GString::new();
    assert_eq!(s.chars_checked(), &[]);
    assert_eq!(unsafe { s.chars_unchecked() }, &[]);
}

#[itest]
fn string_chars() {
    let string = String::from("some_string");
    let string_chars: Vec<char> = string.chars().collect();
    let gstring = GString::from(string);
    let gstring_chars: Vec<char> = gstring.chars_checked().to_vec();

    assert_eq!(gstring_chars, string_chars);
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
fn gstring_name_into_string_sys() {
    const TARGET_STRINGS: &[&'static str] = &[
        "property_number_one",
        "another property here",
        "wow properties",
        "odakfhgjlk",
        "more stuffsies",
    ];
    let mut strings = Vec::new();

    for i in 0..100 {
        let string = TARGET_STRINGS[i % TARGET_STRINGS.len()];
        strings.push(GString::from(string).into_string_sys());
    }

    for (i, string_sys) in strings.iter().enumerate() {
        let target = TARGET_STRINGS[i % TARGET_STRINGS.len()];
        let string = unsafe { GString::from_string_sys(*string_sys) };
        assert_eq!(string.to_string().as_str(), target, "iteration: {i}",);
    }
}
