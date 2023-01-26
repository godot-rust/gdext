/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::collections::{HashMap, HashSet};

use crate::itest;
use godot::{
    builtin::{dict, Dictionary, FromVariant, ToVariant},
    prelude::{Share, Variant},
};

pub fn run() -> bool {
    let mut ok = true;
    ok &= dictionary_default();
    ok &= dictionary_new();
    ok &= dictionary_from_iterator();
    ok &= dictionary_from();
    ok &= dictionary_macro();
    ok &= dictionary_try_to_hashmap();
    ok &= dictionary_try_to_hashset();
    ok &= dictionary_clone();
    ok &= dictionary_duplicate_deep();
    ok &= dictionary_hash();
    ok &= dictionary_get();
    ok &= dictionary_insert();
    ok &= dictionary_insert_multiple();
    ok &= dictionary_insert_long();
    ok &= dictionary_extend();
    ok &= dictionary_remove();
    ok &= dictionary_clear();
    ok &= dictionary_find_key();
    ok &= dictionary_contains_keys();
    ok &= dictionary_keys_values();
    ok &= dictionary_equal();
    ok
}

#[itest]
fn dictionary_default() {
    assert_eq!(Dictionary::default().len(), 0);
}

#[itest]
fn dictionary_new() {
    assert_eq!(Dictionary::new().len(), 0);
}

#[itest]
fn dictionary_from_iterator() {
    let dictionary = Dictionary::from_iter([("foo", 1), ("bar", 2)]);

    assert_eq!(dictionary.len(), 2);
    assert_eq!(dictionary.get("foo"), Some(1.to_variant()), "key = \"foo\"");
    assert_eq!(dictionary.get("bar"), Some(2.to_variant()), "key = \"bar\"");

    let dictionary = Dictionary::from_iter([(1, "foo"), (2, "bar")]);

    assert_eq!(dictionary.len(), 2);
    assert_eq!(dictionary.get(1), Some("foo".to_variant()), "key = 1");
    assert_eq!(dictionary.get(2), Some("bar".to_variant()), "key = 2");
}

#[itest]
fn dictionary_from() {
    let dictionary = Dictionary::from(&HashMap::from([("foo", 1), ("bar", 2)]));

    assert_eq!(dictionary.len(), 2);
    assert_eq!(dictionary.get("foo"), Some(1.to_variant()), "key = \"foo\"");
    assert_eq!(dictionary.get("bar"), Some(2.to_variant()), "key = \"bar\"");

    let dictionary = Dictionary::from(&HashMap::from([(1, "foo"), (2, "bar")]));

    assert_eq!(dictionary.len(), 2);
    assert_eq!(dictionary.get(1), Some("foo".to_variant()), "key = \"foo\"");
    assert_eq!(dictionary.get(2), Some("bar".to_variant()), "key = \"bar\"");
}

#[itest]
fn dictionary_macro() {
    let dictionary = dict! {
        "foo": 0,
        "bar": true,
        "baz": "foobar"
    };

    assert_eq!(dictionary.len(), 3);
    assert_eq!(dictionary.get("foo"), Some(0.to_variant()), "key = \"foo\"");
    assert_eq!(
        dictionary.get("bar"),
        Some(true.to_variant()),
        "key = \"bar\""
    );
    assert_eq!(
        dictionary.get("baz"),
        Some("foobar".to_variant()),
        "key = \"baz\""
    );

    let empty = dict!();
    assert!(empty.is_empty());

    let foo = "foo";
    let dict_complex = dict! {
        foo: 10,
        "bar": true,
        (1 + 2): Variant::nil(),
    };
    assert_eq!(dict_complex.get("foo"), Some(10.to_variant()));
    assert_eq!(dict_complex.get("bar"), Some(true.to_variant()));
    assert_eq!(dict_complex.get(3), Some(Variant::nil()));
}

#[itest]
fn dictionary_try_to_hashmap() {
    let dictionary = dict! {
        "foo": 0,
        "bar": 1,
        "baz": 2
    };

    assert_eq!(
        HashMap::<String, i64>::try_from(&dictionary),
        Ok(HashMap::from([
            ("foo".into(), 0),
            ("bar".into(), 1),
            ("baz".into(), 2)
        ]))
    );
}

#[itest]
fn dictionary_try_to_hashset() {
    let dictionary = dict! {
        "foo": true,
        "bar": true,
        "baz": true
    };

    assert_eq!(
        HashSet::<String>::try_from(&dictionary),
        Ok(HashSet::from(["foo".into(), "bar".into(), "baz".into()]))
    );
}

#[itest]
fn dictionary_clone() {
    let subdictionary = dict! {
        "baz": true,
        "foobar": false
    };
    let dictionary = dict! {
        "foo": 0,
        "bar": subdictionary.share()
    };
    #[allow(clippy::redundant_clone)]
    let clone = dictionary.share();
    Dictionary::try_from_variant(&clone.get("bar").unwrap())
        .unwrap()
        .insert("final", 4);
    assert_eq!(subdictionary.get("final"), Some(4.to_variant()));
}

#[itest]
fn dictionary_hash() {
    let dictionary = dict! {
        "foo": 0,
        "bar": true,
        "baz": "foobar"
    };
    dictionary.hash();
}

#[itest]
fn dictionary_duplicate_deep() {
    let subdictionary = dict! {
        "baz": true,
        "foobar": false
    };
    let dictionary = dict! {
        "foo": 0,
        "bar": subdictionary.share()
    };
    let clone = dictionary.duplicate_deep();
    Dictionary::try_from_variant(&clone.get("bar").unwrap())
        .unwrap()
        .insert("baz", 4);
    assert_eq!(
        subdictionary.get("baz"),
        Some(true.to_variant()),
        "key = \"baz\""
    );
}

#[itest]
fn dictionary_duplicate_shallow() {
    let subdictionary = dict! {
        "baz": true,
        "foobar": false
    };
    let dictionary = dict! {
        "foo": 0,
        "bar": subdictionary.share()
    };
    let mut clone = dictionary.duplicate_shallow();
    Dictionary::try_from_variant(&clone.get("bar").unwrap())
        .unwrap()
        .insert("baz", 4);
    assert_eq!(
        subdictionary.get("baz"),
        Some(4.to_variant()),
        "key = \"baz\""
    );
    clone.insert("foo", false.to_variant());
    assert_eq!(dictionary.get("foo"), Some(0.to_variant()));
    assert_eq!(clone.get("foo"), Some(false.to_variant()));
}

#[itest]
fn dictionary_get() {
    let mut dictionary = dict! {
        "foo": 0,
        "bar": true,
        "baz": "foobar",
        "nil": Variant::nil(),
    };

    dictionary.insert("baz", "foobar");

    assert_eq!(dictionary.get("foo"), Some(0.to_variant()), "key = \"foo\"");
    assert_eq!(
        dictionary.get("bar"),
        Some(true.to_variant()),
        "key = \"bar\""
    );
    assert_eq!(dictionary.get("baz"), Some("foobar".to_variant()));
    assert_eq!(dictionary.get("nil"), Some(Variant::nil()), "key = \"nil\"");
    assert_eq!(dictionary.get("missing"), None, "key = \"missing\"");
    assert_eq!(
        dictionary.get_or_nil("nil"),
        Variant::nil(),
        "key = \"nil\""
    );
    assert_eq!(
        dictionary.get_or_nil("missing"),
        Variant::nil(),
        "key = \"missing\""
    );
    assert_eq!(dictionary.get("foobar"), None, "key = \"foobar\"");
}

#[itest]
fn dictionary_insert() {
    let mut dictionary = dict! {
        "foo": 0,
        "bar": 1,
    };

    assert_eq!(dictionary.insert("bar", 2), Some(1.to_variant()));
    assert_eq!(
        HashMap::<String, i64>::try_from(&dictionary),
        Ok(HashMap::from([("foo".into(), 0), ("bar".into(), 2)]))
    );
    assert_eq!(dictionary.insert("baz", 3), None);
    assert_eq!(
        HashMap::<String, i64>::try_from(&dictionary),
        Ok(HashMap::from([
            ("foo".into(), 0),
            ("bar".into(), 2),
            ("baz".into(), 3)
        ]))
    );
}

#[itest]
fn dictionary_insert_multiple() {
    let mut dictionary = dict! {};
    assert!(dictionary.is_empty());

    dictionary.insert(1, true);
    assert_eq!(dictionary.get(1), Some(true.to_variant()));

    let mut other = dict! {};
    assert!(other.is_empty());

    other.insert(1, 2);
    assert_eq!(other.get(1), Some(2.to_variant()));
}
#[itest]
fn dictionary_insert_long() {
    let mut dictionary = dict! {};
    let old = dictionary.insert("abcdefghijklmnopqrstuvwxyz", "zabcdefghijklmnopqrstuvwxy");
    assert_eq!(old, None);
    assert_eq!(
        dictionary.get("abcdefghijklmnopqrstuvwxyz"),
        Some("zabcdefghijklmnopqrstuvwxy".to_variant())
    );
}

#[itest]
fn dictionary_extend() {
    let mut dictionary = dict! {
        "foo": 0,
        "bar": true,
    };
    assert_eq!(dictionary.get("foo"), Some(0.to_variant()));
    let other = dict! {
        "bar": "new",
        "baz": Variant::nil(),
    };
    dictionary.extend_dictionary(other, false);
    assert_eq!(dictionary.get("bar"), Some(true.to_variant()));
    assert_eq!(dictionary.get("baz"), Some(Variant::nil()));

    let mut dictionary = dict! {
        "bar": true,
    };
    let other = dict! {
        "bar": "new",
    };
    dictionary.extend_dictionary(other, true);
    assert_eq!(dictionary.get("bar"), Some("new".to_variant()));
}

#[itest]
fn dictionary_remove() {
    let mut dictionary = dict! {
        "foo": 0,
    };
    assert_eq!(dictionary.remove("foo"), Some(0.to_variant()));
    assert!(!dictionary.contains_key("foo"));
    assert!(dictionary.is_empty());
}

#[itest]
fn dictionary_clear() {
    let mut dictionary = dict! {
        "foo": 0,
        "bar": true,
        "baz": "foobar"
    };

    assert!(!dictionary.is_empty());
    dictionary.clear();
    assert!(dictionary.is_empty());
}

#[itest]
fn dictionary_find_key() {
    let dictionary = dict! {
        "foo": 0,
        "bar": true,
    };

    assert_eq!(dictionary.find_key_by_value(0), Some("foo".to_variant()));
    assert_eq!(dictionary.find_key_by_value(true), Some("bar".to_variant()));
}

#[itest]
fn dictionary_contains_keys() {
    use godot::prelude::Array;
    let dictionary = dict! {
        "foo": 0,
        "bar": true,
    };

    assert!(dictionary.contains_key("foo"), "key = \"foo\"");
    assert!(dictionary.contains_key("bar"), "key = \"bar\"");
    assert!(
        dictionary.contains_all_keys(Array::from(&["foo", "bar"])),
        "keys = [\"foo\", \"bar\"]"
    );
    assert!(!dictionary.contains_key("missing"), "key = \"missing\"");
    assert!(
        !dictionary.contains_all_keys(Array::from(&["foo", "bar", "missing"])),
        "keys = [\"foo\", \"bar\", \"missing\"]"
    );
}

#[itest]
fn dictionary_keys_values() {
    use godot::prelude::Array;
    let dictionary = dict! {
        "foo": 0,
        "bar": true,
    };

    assert_eq!(dictionary.keys(), Array::from(&["foo", "bar"]));
    assert_eq!(
        dictionary.values(),
        Array::from(&[0.to_variant(), true.to_variant()])
    );
}

#[itest]
fn dictionary_equal() {
    assert_eq!(dict! {"foo": "bar"}, dict! {"foo": "bar"});
    assert_eq!(dict! {1: f32::NAN}, dict! {1: f32::NAN}); // yes apparently godot considers these equal
    assert_ne!(dict! {"foo": "bar"}, dict! {"bar": "foo"});
}
