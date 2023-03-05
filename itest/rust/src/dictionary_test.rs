/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::collections::{HashMap, HashSet};

use crate::{expect_panic, itest};
use godot::builtin::{dict, varray, Dictionary, FromVariant, ToVariant, Variant};
use godot::obj::Share;

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
    Dictionary::from_variant(&clone.get("bar").unwrap()).insert("final", 4);
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
    Dictionary::from_variant(&clone.get("bar").unwrap()).insert("baz", 4);
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
    Dictionary::from_variant(&clone.get("bar").unwrap()).insert("baz", 4);
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
        dictionary
            .iter_shared()
            .typed::<String, i64>()
            .collect::<HashMap<_, _>>(),
        HashMap::from([("foo".into(), 0), ("bar".into(), 2)])
    );
    assert_eq!(dictionary.insert("baz", 3), None);
    assert_eq!(
        dictionary
            .iter_shared()
            .typed::<String, i64>()
            .collect::<HashMap<_, _>>(),
        HashMap::from([("foo".into(), 0), ("bar".into(), 2), ("baz".into(), 3)])
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
    let dictionary = dict! {
        "foo": 0,
        "bar": true,
    };

    assert!(dictionary.contains_key("foo"), "key = \"foo\"");
    assert!(dictionary.contains_key("bar"), "key = \"bar\"");
    assert!(
        dictionary.contains_all_keys(varray!["foo", "bar"]),
        "keys = [\"foo\", \"bar\"]"
    );
    assert!(!dictionary.contains_key("missing"), "key = \"missing\"");
    assert!(
        !dictionary.contains_all_keys(varray!["foo", "bar", "missing"]),
        "keys = [\"foo\", \"bar\", \"missing\"]"
    );
}

#[itest]
fn dictionary_keys_values() {
    let dictionary = dict! {
        "foo": 0,
        "bar": true,
    };

    assert_eq!(dictionary.keys_array(), varray!["foo", "bar"]);
    assert_eq!(dictionary.values_array(), varray![0, true]);
}

#[itest]
fn dictionary_equal() {
    assert_eq!(dict! {"foo": "bar"}, dict! {"foo": "bar"});
    assert_eq!(dict! {1: f32::NAN}, dict! {1: f32::NAN}); // yes apparently Godot considers these equal
    assert_ne!(dict! {"foo": "bar"}, dict! {"bar": "foo"});
}

#[itest]
fn dictionary_iter() {
    let dictionary = dict! {
        "foo": 0,
        "bar": true,
        "baz": "foobar",
        "nil": Variant::nil(),
    };

    let map = HashMap::<String, Variant>::from([
        ("foo".into(), 0.to_variant()),
        ("bar".into(), true.to_variant()),
        ("baz".into(), "foobar".to_variant()),
        ("nil".into(), Variant::nil()),
    ]);

    let map2: HashMap<String, Variant> = dictionary.iter_shared().typed().collect();
    assert_eq!(map, map2);
}

#[itest]
fn dictionary_iter_equals_big() {
    let dictionary: Dictionary = (0..1000).zip(0..1000).collect();
    let map: HashMap<i64, i64> = (0..1000).zip(0..1000).collect();
    let collected_map: HashMap<i64, i64> = dictionary.iter_shared().typed::<i64, i64>().collect();
    assert_eq!(map, collected_map);
    let collected_dictionary: Dictionary = collected_map.into_iter().collect();
    assert_eq!(dictionary, collected_dictionary);
}

// Insertion mid-iteration seems to work and is not explicitly forbidden in the docs:
// https://docs.godotengine.org/en/latest/classes/class_dictionary.html#description

#[itest]
fn dictionary_iter_insert() {
    let dictionary = dict! {
        "foo": 0,
        "bar": true,
        "baz": "foobar",
        "nil": Variant::nil(),
    };
    let mut dictionary2 = dictionary.share();

    let mut iter = dictionary.iter_shared();
    iter.next();
    iter.next();

    dictionary2.insert("new_key", 10);
    let v: Vec<_> = iter.collect();
    assert_eq!(dictionary.len(), 5);
    assert!(dictionary.contains_key("new_key"));
    assert_eq!(v.len(), 3);
    assert!(v.contains(&("new_key".to_variant(), 10.to_variant())));
}

#[itest]
fn dictionary_iter_insert_after_completion() {
    let dictionary = dict! {
        "foo": 0,
        "bar": true,
        "baz": "foobar",
        "nil": Variant::nil(),
    };
    let mut dictionary2 = dictionary.share();
    let mut iter = dictionary.iter_shared();
    for _ in 0..4 {
        iter.next();
    }
    assert_eq!(iter.next(), None);

    dictionary2.insert("new_key", 10);
    assert_eq!(iter.next(), None);
    assert_eq!(dictionary.len(), 5);
}

#[itest]
fn dictionary_iter_big() {
    let dictionary: Dictionary = (0..256).zip(0..256).collect();
    let mut dictionary2 = dictionary.share();
    let mut iter = dictionary.iter_shared();

    for _ in 0..4 {
        for _ in 0..4 {
            for _ in 0..16 {
                iter.next();
            }
            dictionary2.insert("a", "b");
        }
        dictionary2.clear();
        dictionary2.extend((0..64).zip(0..64));
    }
    assert_eq!(dictionary2, (0..64).zip(0..64).collect());
}

#[itest]
fn dictionary_iter_simultaneous() {
    let dictionary = dict! {
        "foo": 10,
        "bar": true,
        "baz": "foobar",
        "nil": Variant::nil(),
    };

    let map: HashMap<String, (Variant, Variant)> = dictionary
        .iter_shared()
        .typed::<String, Variant>()
        .zip(dictionary.iter_shared().typed::<String, Variant>())
        .map(|((mut k1, v1), (k2, v2))| {
            k1.push_str(k2.as_str());
            (k1, (v1, v2))
        })
        .collect();

    assert!(map.len() == 4);

    let mut tens = 0;
    let mut trues = 0;
    let mut foobars = 0;
    let mut nils = 0;

    for v in map.iter().flat_map(|(_, (v1, v2))| [v1, v2]) {
        if let Ok(b) = bool::try_from_variant(v) {
            assert!(b);
            trues += 1;
        } else if let Ok(i) = i64::try_from_variant(v) {
            assert_eq!(i, 10);
            tens += 1;
        } else if let Ok(s) = String::try_from_variant(v) {
            assert_eq!(s.as_str(), "foobar");
            foobars += 1;
        } else {
            assert!(v.is_nil());
            nils += 1;
        }
    }

    assert_eq!(tens, 2);
    assert_eq!(trues, 2);
    assert_eq!(foobars, 2);
    assert_eq!(nils, 2);
}

#[itest]
fn dictionary_iter_panics() {
    expect_panic(
        "Dictionary containing integer keys should not be convertible to a HashSet<String>",
        || {
            let dictionary: Dictionary = (0..10).zip(0..).collect();
            let _set: HashSet<String> = dictionary.keys_shared().typed::<String>().collect();
        },
    );

    expect_panic(
        "Dictionary containing integer entries should not be convertible to a HashMap<String,String>",
        || {
            let dictionary: Dictionary = (0..10).zip(0..).collect();
            let _set: HashMap<String,String> = dictionary.iter_shared().typed::<String,String>().collect();
        },
    );
}

// The below tests erase entries mid-iteration. This is not supported by Godot dictionaries
// however it shouldn't cause unsafety or panicking. Rather the outcome of the iteration is not
// guaranteed. These tests therefore test two main things:
// 1. The dictionary is not corrupted by erasing mid-iteration.
// 2. Our implementation behaves the same as Godot.
//
// #2 may change in the future, so equivalent GDScript code is provided. That way we can
// easily check if a failure is a false negative caused by Godot's behavior changing.

#[itest]
fn dictionary_iter_clear() {
    let dictionary = dict! {
        "foo": 0,
        "bar": true,
        "baz": "foobar",
        "nil": Variant::nil(),
    };
    let mut dictionary2 = dictionary.share();

    let mut iter = dictionary.iter_shared();
    iter.next();
    iter.next();
    dictionary2.clear();
    let v: Vec<_> = iter.collect();
    assert!(dictionary.is_empty(), "Dictionary contains {dictionary:?}.");
    assert!(v.is_empty(), "Vec contains {v:?}.");
    /* equivalent GDScript code:
    ```
    var dictionary = {
        "foo": 0,
        "bar": true,
        "baz": "foobar",
        "nil": null,
    }
    var arr = []

    var i = 0
    for key in dictionary:
        var value = dictionary.get(key)
        if i == 1:
            dictionary.clear()
        elif i > 1:
            arr.append([key, value])
        i += 1
    print(dictionary)
    print(arr)
    ```
     */
}

#[itest]
fn dictionary_iter_erase() {
    let dictionary = dict! {
        "foo": 0,
        "bar": true,
        "baz": "foobar",
        "nil": Variant::nil(),
    };
    let mut dictionary2 = dictionary.share();

    let mut iter = dictionary.iter_shared();
    iter.next();
    iter.next();
    dictionary2.remove("baz");
    let v: Vec<_> = iter.collect();
    assert_eq!(dictionary.len(), 3);
    assert_eq!(v.len(), 1);
    assert!(v.contains(&("nil".to_variant(), Variant::nil())));
    /* equivalent GDScript code:
    ```
    var dictionary = {
        "foo": 0,
        "bar": true,
        "baz": "foobar",
        "nil": null,
    }
    var arr = []

    var i = 0
    for key in dictionary:
        var value = dictionary.get(key)
        if i == 1:
            dictionary.erase("baz")
        elif i > 1:
            arr.append([key, value])
        i += 1
    print(dictionary)
    print(arr)
    ```
     */
}
