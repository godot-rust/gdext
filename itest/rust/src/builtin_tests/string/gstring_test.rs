/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
use std::borrow::Cow;
use std::collections::HashSet;
use std::fmt;
use std::rc::Rc;

use godot::builtin::{Encoding, GString, GodotStringExt, PackedStringArray};

use super::string_test_macros::{APPLE_CHARS, APPLE_STR};
use crate::framework::{expect_panic_or_nothing, itest};

#[itest]
fn gstring_default() {
    let string = GString::new();
    let back = String::from(&string);

    assert_eq!(back.as_str(), "");
}

#[itest]
fn gstring_conversion() {
    let string = String::from("some string");
    let second = GString::from(&string);
    let back = String::from(&second);

    assert_eq!(string, back);
}

#[itest]
fn gstring_equality() {
    let string = GString::from("some string");
    let second = GString::from("some string");
    let different = GString::from("some");

    assert_eq!(string, second);
    assert_ne!(string, different);
}

#[itest]
fn string_eq_str() {
    let gstring = GString::from("hello");
    assert_eq!(gstring, "hello");
    assert_ne!(gstring, "hallo");
}

#[itest]
fn gstring_ordering() {
    let low = GString::from("Alpha");
    let high = GString::from("Beta");

    assert!(low < high);
    assert!(low <= high);
    assert!(high > low);
    assert!(high >= low);
}

#[itest]
fn gstring_clone() {
    let first = GString::from("some string");
    #[allow(clippy::redundant_clone)]
    let cloned = first.clone();

    assert_eq!(first, cloned);
}

#[itest]
fn gstring_chars() {
    // Empty tests regression from #228: Null pointer passed to slice::from_raw_parts().
    let string = GString::new();
    let empty_char_slice: &[char] = &[];
    assert_eq!(string.chars(), empty_char_slice);
    assert_eq!(string, GString::from(empty_char_slice));

    let string = String::from(APPLE_STR);
    let string_chars: Vec<char> = string.chars().collect();
    let gstring = GString::from(&string);

    assert_eq!(gstring.chars(), string_chars.as_slice());
    assert_eq!(gstring.chars(), APPLE_CHARS);

    assert_eq!(gstring, GString::from(string_chars.as_slice()));
}

#[itest]
fn gstring_unicode_at() {
    let s = GString::from(APPLE_STR);
    assert_eq!(s.unicode_at(0), 'ö');
    assert_eq!(s.unicode_at(1), '🍎');
    assert_eq!(s.unicode_at(2), 'A');
    assert_eq!(s.unicode_at(3), '💡');

    // Release mode: out-of-bounds prints Godot error, but returns 0.
    expect_panic_or_nothing("unicode_at() out-of-bounds panics", || {
        assert_eq!(s.unicode_at(4), '\0');
    });
}

#[itest]
fn gstring_hash() {
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
fn gstring_with_null() {
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

#[itest]
fn gstring_substr() {
    let string = GString::from("stable");
    assert_eq!(string.substr(..), "stable");
    assert_eq!(string.substr(1..), "table");
    assert_eq!(string.substr(..4), "stab");
    assert_eq!(string.substr(..=3), "stab");
    assert_eq!(string.substr(2..5), "abl");
    assert_eq!(string.substr(2..=4), "abl");
}

#[itest]
fn gstring_find() {
    let s = GString::from("Hello World");

    assert_eq!(s.find("o"), Some(4));

    // Forward
    assert_eq!(s.find_ex("o").done(), Some(4));
    assert_eq!(s.find_ex("O").done(), None);
    assert_eq!(s.find_ex("O").n().done(), Some(4));
    assert_eq!(s.find_ex("O").n().from(4).done(), Some(4));
    assert_eq!(s.find_ex("O").n().from(5).done(), Some(7));

    // Reverse
    assert_eq!(s.find_ex("o").r().done(), Some(7));
    assert_eq!(s.find_ex("O").r().done(), None);
    assert_eq!(s.find_ex("O").r().n().done(), Some(7));
    assert_eq!(s.find_ex("O").r().n().from(7).done(), Some(7));
    assert_eq!(s.find_ex("O").r().n().from(6).done(), Some(4));
}

#[itest]
fn gstring_split() {
    let s = GString::from("Hello World");
    assert_eq!(s.split(" "), packed(&["Hello", "World"]));
    assert_eq!(
        s.split(""),
        packed(&["H", "e", "l", "l", "o", " ", "W", "o", "r", "l", "d"])
    );
    assert_eq!(s.split_ex(" ").done(), packed(&["Hello", "World"]));
    assert_eq!(s.split_ex("world").done(), packed(&["Hello World"]));

    // Empty divisions
    assert_eq!(s.split_ex("l").done(), packed(&["He", "", "o Wor", "d"]));
    assert_eq!(
        s.split_ex("l").disallow_empty().done(),
        packed(&["He", "o Wor", "d"])
    );

    // Max-split
    assert_eq!(
        s.split_ex("l").maxsplit(1).done(),
        packed(&["He", "lo World"])
    );
    assert_eq!(
        s.split_ex("l").maxsplit(2).done(),
        packed(&["He", "", "o World"])
    );

    // Reverse max-split
    assert_eq!(
        s.split_ex("l").maxsplit_r(1).done(),
        packed(&["Hello Wor", "d"])
    );
}

#[itest]
fn gstring_count() {
    let s = GString::from("Long sentence with Sentry guns.");
    assert_eq!(s.count("sent", ..), 1);
    assert_eq!(s.count("en", 6..), 3);
    assert_eq!(s.count("en", 7..), 2);
    assert_eq!(s.count("en", 6..=6), 0);
    assert_eq!(s.count("en", 6..=7), 1);
    assert_eq!(s.count("en", 6..8), 1);
    assert_eq!(s.count("en", 7..8), 0);
    assert_eq!(s.count("en", ..8), 1);
    assert_eq!(s.count("en", ..10), 1);
    assert_eq!(s.count("en", ..11), 2);
    assert_eq!(s.count("en", ..=10), 2);

    assert_eq!(s.countn("sent", ..), 2);
}

#[itest]
fn gstring_erase() {
    let s = GString::from("Hello World");
    assert_eq!(s.erase(..), GString::new());
    assert_eq!(s.erase(4..4), s);
    assert_eq!(s.erase(2..=2), "Helo World");
    assert_eq!(s.erase(1..=3), "Ho World");
    assert_eq!(s.erase(1..4), "Ho World");
    assert_eq!(s.erase(..6), "World");
    assert_eq!(s.erase(5..), "Hello");
}

#[itest]
fn gstring_insert() {
    let s = GString::from("H World");
    assert_eq!(s.insert(1, "i"), "Hi World");
    assert_eq!(s.insert(1, "ello"), "Hello World");
    assert_eq!(s.insert(7, "."), "H World.");
    assert_eq!(s.insert(0, "¿"), "¿H World");

    // Special behavior in Godot, but maybe the idea is to allow large constants to mean "end".
    assert_eq!(s.insert(123, "!"), "H World!");
}

#[itest]
fn gstring_pad() {
    let s = GString::from("123");
    assert_eq!(s.lpad(5, '0'), "00123");
    assert_eq!(s.lpad(2, ' '), "123");
    assert_eq!(s.lpad(4, ' '), " 123");

    assert_eq!(s.rpad(5, '+'), "123++");
    assert_eq!(s.rpad(2, ' '), "123");
    assert_eq!(s.rpad(4, ' '), "123 ");

    let s = GString::from("123.456");
    assert_eq!(s.pad_decimals(5), "123.45600");
    assert_eq!(s.pad_decimals(2), "123.45"); // note: Godot rounds down

    assert_eq!(s.pad_zeros(5), "00123.456");
    assert_eq!(s.pad_zeros(2), "123.456");
}

// Byte and C-string conversions.
crate::generate_string_bytes_and_cstr_tests!(
    builtin: GString,
    tests: [
        gstring_from_bytes_ascii,
        gstring_from_cstr_ascii,
        gstring_from_bytes_latin1,
        gstring_from_cstr_latin1,
        gstring_from_bytes_utf8,
        gstring_from_cstr_utf8,
    ]
);

crate::generate_string_standard_fmt_tests!(
    builtin: GString,
    tests: [
        gstring_display,
        gstring_standard_pad,
    ]
);

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Helpers

fn packed(strings: &[&str]) -> PackedStringArray {
    strings.iter().map(|&s| GString::from(s)).collect()
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// GodotStringExt roundtrips

#[itest]
// Ignore warnings -- explicitly test Box<str>, &[char], Vec<char> auto-deref.
#[expect(unused_allocation, clippy::needless_borrow, clippy::useless_vec)]
fn string_ext_conversion() {
    let utf8 = String::from("godöt-rust 🦀 ゴドー");
    let utf8: &str = utf8.as_str(); // Ensure that non-'static lifetime is supported.

    assert_eq!(utf8.to_gstring(), utf8);
    assert_eq!(utf8.to_string_name(), utf8);
    assert_eq!(utf8.to_node_path().to_string(), utf8); // NodePath == &str operator is not supported.

    // Standard types converted to GString.
    assert_eq!(utf8.to_gstring(), utf8); // &str
    assert_eq!(Cow::Borrowed(utf8).to_gstring(), utf8); // String
    assert_eq!(String::from(utf8).to_gstring(), utf8); // Cow<'_, str>
    assert_eq!(Box::new(utf8).to_gstring(), utf8); // Box<str>
    assert_eq!(Rc::new(utf8).to_gstring(), utf8); // Rc<str>
    assert_eq!(['ĝ', 'ド'].to_gstring(), "ĝド"); // [char]
    assert_eq!((&['ĝ', 'ド']).to_gstring(), "ĝド"); // &[char]
    assert_eq!(vec!['ĝ', 'ド'].to_gstring(), "ĝド"); // Vec<char>
}

#[itest]
fn string_ext_char_conversion() {
    let utf32: &[char] = &['G', 'ゴ', ' ', '🦀'];
    assert_eq!(utf32.to_gstring().chars(), utf32);

    // StringName::chars() only in >= 4.5; fall back via to_string().
    #[cfg(since_api = "4.5")]
    assert_eq!(utf32.to_string_name().chars(), utf32);
    #[cfg(before_api = "4.5")]
    {
        let np_string = utf32.to_string_name().to_string();
        assert_eq!(np_string.chars().collect::<Vec<char>>(), utf32);
    }

    // NodePath has no chars() function, so use to_string().
    let np_string = utf32.to_node_path().to_string();
    assert_eq!(np_string.chars().collect::<Vec<char>>(), utf32);
}

#[itest]
fn string_ext_bounds() {
    let utf8 = String::from("godöt-rust 🦀 ゴドー");
    let utf8: &str = utf8.as_str(); // Ensure that non-'static lifetime is supported.

    check_string_ext(utf8.to_gstring());
    check_string_ext(utf8.to_string_name());
    check_string_ext(utf8.to_node_path());

    // Standard string types currently not supported as Sized bound. Could be enabled if we add such impls in the future.
}

// Tests generic bounds (currently only Sized impls).
fn check_string_ext<T: fmt::Display + GodotStringExt>(value: T) {
    let expected = value.to_string();
    assert_eq!(value.to_gstring().to_string(), expected);
    assert_eq!(value.to_string_name().to_string(), expected);
    assert_eq!(value.to_node_path().to_string(), expected);
}
