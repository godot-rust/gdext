/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::collections::{HashMap, HashSet};

use godot::builtin::{varray, vdict, Dictionary, Variant};
use godot::meta::{FromGodot, ToGodot};
use godot::sys::GdextBuild;

use crate::framework::{expect_panic, itest};

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
    let dictionary = vdict! {
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

    let empty = vdict!();
    assert!(empty.is_empty());

    let key = "num";
    let dict_complex = vdict! {
        key: 10,
        "bool": true,
        (1 + 2): Variant::nil(),
    };
    assert_eq!(dict_complex.get("num"), Some(10.to_variant()));
    assert_eq!(dict_complex.get("bool"), Some(true.to_variant()));
    assert_eq!(dict_complex.get(3), Some(Variant::nil()));
}

#[itest]
fn dictionary_clone() {
    let subdictionary = vdict! {
        "baz": true,
        "foobar": false
    };
    let dictionary = vdict! {
        "foo": 0,
        "bar": subdictionary.clone()
    };

    #[allow(clippy::redundant_clone)]
    let clone = dictionary.clone();
    Dictionary::from_variant(&clone.get("bar").unwrap()).set("final", 4);
    assert_eq!(subdictionary.get("final"), Some(4.to_variant()));
}

#[itest]
fn dictionary_hash() {
    use godot::builtin::Vector2i;

    let a = vdict! {
        "foo": 0,
        "bar": true,
        (Vector2i::new(4, -1)): "foobar",
    };
    let b = vdict! {
        "foo": 0,
        "bar": true,
        (Vector2i::new(4, -1)): "foobar" // No comma to test macro.
    };
    let c = vdict! {
        "foo": 0,
        (Vector2i::new(4, -1)): "foobar",
        "bar": true,
    };

    assert_eq!(a.hash(), b.hash(), "equal dictionaries have same hash");
    assert_ne!(
        a.hash(),
        c.hash(),
        "dictionaries with reordered content have different hash"
    );

    // NaNs are not equal (since Godot 4.2) but share same hash.
    assert_eq!(vdict! {772: f32::NAN}.hash(), vdict! {772: f32::NAN}.hash());
}

#[itest]
fn dictionary_duplicate_deep() {
    let subdictionary = vdict! {
        "baz": true,
        "foobar": false
    };
    let dictionary = vdict! {
        "foo": 0,
        "bar": subdictionary.clone()
    };
    let clone = dictionary.duplicate_deep();
    Dictionary::from_variant(&clone.get("bar").unwrap()).set("baz", 4);
    assert_eq!(
        subdictionary.get("baz"),
        Some(true.to_variant()),
        "key = \"baz\""
    );
}

#[itest]
fn dictionary_duplicate_shallow() {
    let subdictionary = vdict! {
        "baz": true,
        "foobar": false
    };
    let dictionary = vdict! {
        "foo": 0,
        "bar": subdictionary.clone()
    };

    let mut clone = dictionary.duplicate_shallow();
    Dictionary::from_variant(&clone.get("bar").unwrap()).set("baz", 4);
    assert_eq!(
        subdictionary.get("baz"),
        Some(4.to_variant()),
        "key = \"baz\""
    );

    clone.set("foo", false);
    assert_eq!(dictionary.get("foo"), Some(0.to_variant()));
    assert_eq!(clone.get("foo"), Some(false.to_variant()));
}

#[itest]
fn dictionary_get() {
    let dictionary = vdict! {
        "foo": 0,
        "bar": true,
        "baz": "foobar",
        "nil": Variant::nil(),
    };

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
fn dictionary_at() {
    let dictionary = vdict! {
        "foo": 0,
        "baz": "foobar",
        "nil": Variant::nil(),
    };

    assert_eq!(dictionary.at("foo"), 0.to_variant(), "key = \"foo\"");
    assert_eq!(dictionary.at("baz"), "foobar".to_variant(), "key = \"baz\"");
    assert_eq!(dictionary.at("nil"), Variant::nil(), "key = \"nil\"");
    expect_panic("key = \"bar\"", || {
        dictionary.at("bar");
    });
}

#[itest]
fn dictionary_set() {
    let mut dictionary = vdict! { "zero": 0, "one": 1 };

    dictionary.set("zero", 2);
    assert_eq!(dictionary, vdict! { "zero": 2, "one": 1 });
}

#[itest]
fn dictionary_set_readonly() {
    let mut dictionary = vdict! { "zero": 0, "one": 1 }.into_read_only();

    #[cfg(debug_assertions)]
    expect_panic("Mutating read-only dictionary in Debug mode", || {
        dictionary.set("zero", 2);
    });

    #[cfg(not(debug_assertions))]
    dictionary.set("zero", 2); // silently fails.

    assert_eq!(dictionary.at("zero"), 0.to_variant());
}

#[itest]
fn dictionary_insert() {
    let mut dictionary = vdict! {
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
    let mut dictionary = vdict! {};
    assert!(dictionary.is_empty());

    dictionary.set(1, true);
    assert_eq!(dictionary.get(1), Some(true.to_variant()));

    let mut other = vdict! {};
    assert!(other.is_empty());

    other.set(1, 2);
    assert_eq!(other.get(1), Some(2.to_variant()));
}
#[itest]
fn dictionary_insert_long() {
    let mut dictionary = vdict! {};
    let old = dictionary.insert("abcdefghijklmnopqrstuvwxyz", "zabcdefghijklmnopqrstuvwxy");
    assert_eq!(old, None);
    assert_eq!(
        dictionary.get("abcdefghijklmnopqrstuvwxyz"),
        Some("zabcdefghijklmnopqrstuvwxy".to_variant())
    );
}

#[itest]
fn dictionary_extend() {
    let mut dictionary = vdict! {
        "foo": 0,
        "bar": true,
    };
    assert_eq!(dictionary.get("foo"), Some(0.to_variant()));
    let other = vdict! {
        "bar": "new",
        "baz": Variant::nil(),
    };
    dictionary.extend_dictionary(&other, false);
    assert_eq!(dictionary.get("bar"), Some(true.to_variant()));
    assert_eq!(dictionary.get("baz"), Some(Variant::nil()));

    let mut dictionary = vdict! {
        "bar": true,
    };
    let other = vdict! {
        "bar": "new",
    };
    dictionary.extend_dictionary(&other, true);
    assert_eq!(dictionary.get("bar"), Some("new".to_variant()));
}

#[itest]
fn dictionary_remove() {
    let mut dictionary = vdict! {
        "foo": 0,
    };
    assert_eq!(dictionary.remove("foo"), Some(0.to_variant()));
    assert!(!dictionary.contains_key("foo"));
    assert!(dictionary.is_empty());
}

#[itest]
fn dictionary_clear() {
    let mut dictionary = vdict! {
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
    let dictionary = vdict! {
        "foo": 0,
        "bar": true,
    };

    assert_eq!(dictionary.find_key_by_value(0), Some("foo".to_variant()));
    assert_eq!(dictionary.find_key_by_value(true), Some("bar".to_variant()));
}

#[itest]
fn dictionary_contains_keys() {
    let dictionary = vdict! {
        "foo": 0,
        "bar": true,
    };

    assert!(dictionary.contains_key("foo"), "key = \"foo\"");
    assert!(dictionary.contains_key("bar"), "key = \"bar\"");
    assert!(
        dictionary.contains_all_keys(&varray!["foo", "bar"]),
        "keys = [\"foo\", \"bar\"]"
    );
    assert!(!dictionary.contains_key("missing"), "key = \"missing\"");
    assert!(
        !dictionary.contains_all_keys(&varray!["foo", "bar", "missing"]),
        "keys = [\"foo\", \"bar\", \"missing\"]"
    );
}

#[itest]
fn dictionary_keys_values() {
    let dictionary = vdict! {
        "foo": 0,
        "bar": true,
    };

    assert_eq!(dictionary.keys_array(), varray!["foo", "bar"]);
    assert_eq!(dictionary.values_array(), varray![0, true]);
}

#[itest]
fn dictionary_equal() {
    assert_eq!(vdict! {"foo": "bar"}, vdict! {"foo": "bar"});
    assert_ne!(vdict! {"foo": "bar"}, vdict! {"bar": "foo"});

    // Changed in https://github.com/godotengine/godot/pull/74588.
    if GdextBuild::before_api("4.2") {
        assert_eq!(vdict! {1: f32::NAN}, vdict! {1: f32::NAN});
    } else {
        assert_ne!(vdict! {1: f32::NAN}, vdict! {1: f32::NAN});
    }
}

#[itest]
fn dictionary_iter() {
    let dictionary = vdict! {
        "foo": 0,
        "bar": true,
        "baz": "foobar",
        "nil": Variant::nil(),
    };

    let map = HashMap::<String, Variant>::from([
        ("foo".to_string(), 0.to_variant()),
        ("bar".to_string(), true.to_variant()),
        ("baz".to_string(), "foobar".to_variant()),
        ("nil".to_string(), Variant::nil()),
    ]);

    let map2: HashMap<String, Variant> = dictionary.iter_shared().typed().collect();
    assert_eq!(map, map2);
}

#[itest]
fn dictionary_iter_size_hint() {
    // Test a completely empty dict.
    let dictionary = Dictionary::new();
    let iter = dictionary.iter_shared();
    assert_eq!(iter.size_hint(), (0, Some(0)));

    // Test a full dictionary being emptied.
    let dictionary = vdict! {
        "foo": 0,
        "bar": true,
        "baz": "foobar",
        "nil": Variant::nil(),
    };

    let mut dictionary_clone = dictionary.clone();
    let mut iter = dictionary.iter_shared();
    assert_eq!(iter.size_hint(), (4, Some(4)));

    iter.next();
    iter.next();
    iter.next();
    assert_eq!(iter.size_hint(), (1, Some(1)));

    iter.next();
    assert_eq!(iter.size_hint(), (0, Some(0)));

    iter.next();
    assert_eq!(iter.size_hint(), (0, Some(0)));

    // Insertion while iterating is allowed and might change size hint.
    dictionary_clone.set("new_key", "soma_val");
    assert_eq!(iter.size_hint(), (1, Some(1)));

    // Removal while iterating is also allowed and might change size_hint.
    dictionary_clone.remove("new_key");
    assert_eq!(iter.size_hint(), (0, Some(0)));
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
    let dictionary = vdict! {
        "foo": 0,
        "bar": true,
        "baz": "foobar",
        "nil": Variant::nil(),
    };
    let mut dictionary2 = dictionary.clone();

    let mut iter = dictionary.iter_shared();
    iter.next();
    iter.next();

    let prev = dictionary2.insert("new_key", 10);
    assert_eq!(prev, None);

    let v: Vec<_> = iter.collect();
    assert_eq!(dictionary.len(), 5);
    assert!(dictionary.contains_key("new_key"));
    assert_eq!(v.len(), 3);
    assert!(v.contains(&("new_key".to_variant(), 10.to_variant())));
}

#[itest]
fn dictionary_iter_insert_after_completion() {
    let dictionary = vdict! {
        "foo": 0,
        "bar": true,
        "baz": "foobar",
        "nil": Variant::nil(),
    };
    let mut dictionary2 = dictionary.clone();
    let mut iter = dictionary.iter_shared();
    for _ in 0..4 {
        iter.next();
    }
    assert_eq!(iter.next(), None);

    dictionary2.set("new_key", 10);
    assert_eq!(iter.next(), None);
    assert_eq!(dictionary.len(), 5);
}

#[itest]
fn dictionary_iter_big() {
    let dictionary: Dictionary = (0..256).zip(0..256).collect();
    let mut dictionary2 = dictionary.clone();
    let mut iter = dictionary.iter_shared();

    for _ in 0..4 {
        for _ in 0..4 {
            for _ in 0..16 {
                iter.next();
            }
            dictionary2.set("a", "b");
        }
        dictionary2.clear();
        dictionary2.extend((0..64).zip(0..64));
    }
    assert_eq!(dictionary2, (0..64).zip(0..64).collect());
}

#[itest]
fn dictionary_iter_simultaneous() {
    let dictionary = vdict! {
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

    assert_eq!(map.len(), 4);

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
    let dictionary = vdict! {
        "foo": 0,
        "bar": true,
        "baz": "foobar",
        "nil": Variant::nil(),
    };
    let mut dictionary2 = dictionary.clone();

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
    let dictionary = vdict! {
        "foo": 0,
        "bar": true,
        "baz": "foobar",
        "nil": Variant::nil(),
    };
    let mut dictionary2 = dictionary.clone();

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

#[itest]
fn dictionary_should_format_with_display() {
    let d = Dictionary::new();
    assert_eq!(format!("{d}"), "{  }");

    let d = vdict! {
        "one": 1,
        "two": true,
        "three": Variant::nil()
    };
    assert_eq!(format!("{d}"), "{ one: 1, two: true, three: <null> }")
}
