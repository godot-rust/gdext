/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::{expect_panic, itest};
use godot::builtin::{Array, FromVariant, GodotString, ToVariant};

pub fn run() -> bool {
    let mut ok = true;
    ok &= array_default();
    ok &= array_new();
    ok &= array_from_iterator();
    ok &= array_from();
    ok &= array_try_to_vec();
    ok &= array_into_iterator();
    ok &= array_clone();
    // ok &= array_duplicate_deep();
    ok &= array_hash();
    ok &= array_get();
    ok &= array_first_last();
    ok &= array_binary_search();
    ok &= array_find();
    ok &= array_rfind();
    ok &= array_min_max();
    ok &= array_set();
    ok &= array_push_pop();
    ok &= array_insert();
    ok &= array_extend();
    ok &= array_reverse();
    ok &= array_sort();
    ok &= array_shuffle();
    ok
}

#[itest]
fn array_default() {
    assert_eq!(Array::default().len(), 0);
}

#[itest]
fn array_new() {
    assert_eq!(Array::new().len(), 0);
}

#[itest]
fn array_from_iterator() {
    let array = Array::from_iter([1, 2]);

    assert_eq!(array.len(), 2);
    assert_eq!(array.get(0), 1.to_variant());
    assert_eq!(array.get(1), 2.to_variant());
}

#[itest]
fn array_from() {
    let array = Array::from(&[1, 2]);

    assert_eq!(array.len(), 2);
    assert_eq!(array.get(0), 1.to_variant());
    assert_eq!(array.get(1), 2.to_variant());
}

#[itest]
fn array_try_to_vec() {
    let array = Array::from(&[1, 2]);
    assert_eq!(array.try_to_vec::<i64>(), Ok(vec![1, 2]));
}

#[itest]
fn array_into_iterator() {
    let array = Array::from(&[1, 2]);
    let mut iter = array.into_iter();
    assert_eq!(iter.next(), Some(1.to_variant()));
    assert_eq!(iter.next(), Some(2.to_variant()));
    assert_eq!(iter.next(), None);
}

#[itest]
fn array_clone() {
    let subarray = Array::from(&[2, 3]);
    let array = Array::from(&[1.to_variant(), subarray.to_variant()]);
    #[allow(clippy::redundant_clone)]
    let clone = array.clone();
    Array::try_from_variant(&clone.get(1))
        .unwrap()
        .set(0, 4.to_variant());
    assert_eq!(subarray.get(0), 4.to_variant());
}

#[itest]
fn array_hash() {
    let array = Array::from(&[1, 2]);
    // Just testing that it converts successfully from i64 to u32.
    array.hash();
}

// TODO: enable once the implementation no longer segfaults
// #[itest]
// fn array_duplicate_deep() {
//     let subarray = Array::from(&[2, 3]);
//     let array = Array::from(&[1.to_variant(), subarray.to_variant()]);
//     let mut clone = array.duplicate_deep();
//     Array::try_from_variant(clone.get(1)).unwrap().set(0, 4.to_variant());
//     assert_eq!(subarray.get(0), 3.to_variant());
// }

#[itest]
fn array_get() {
    let array = Array::from(&[1, 2]);

    assert_eq!(array.get(0), 1.to_variant());
    assert_eq!(array.get(1), 2.to_variant());
    expect_panic("Array index 2 out of bounds: length is 2", || {
        array.get(2);
    });
}

#[itest]
fn array_first_last() {
    let array = Array::from(&[1, 2]);

    assert_eq!(array.first(), Some(1.to_variant()));
    assert_eq!(array.last(), Some(2.to_variant()));

    let empty_array = Array::new();

    assert_eq!(empty_array.first(), None);
    assert_eq!(empty_array.last(), None);
}

#[itest]
fn array_binary_search() {
    let array = Array::from(&[1, 2]);

    assert_eq!(array.binary_search(0.to_variant()), 0);
    assert_eq!(array.binary_search(1.to_variant()), 0);
    assert_eq!(array.binary_search(1.5f64.to_variant()), 1);
    assert_eq!(array.binary_search(2.to_variant()), 1);
    assert_eq!(array.binary_search(3.to_variant()), 2);
}

#[itest]
fn array_find() {
    let array = Array::from(&[1, 2, 1]);

    assert_eq!(array.find(0.to_variant(), None), None);
    assert_eq!(array.find(1.to_variant(), None), Some(0));
    assert_eq!(array.find(1.to_variant(), Some(1)), Some(2));
}

#[itest]
fn array_rfind() {
    let array = Array::from(&[1, 2, 1]);

    assert_eq!(array.rfind(0.to_variant(), None), None);
    assert_eq!(array.rfind(1.to_variant(), None), Some(2));
    assert_eq!(array.rfind(1.to_variant(), Some(1)), Some(0));
}

#[itest]
fn array_min_max() {
    let int_array = Array::from(&[1, 2]);

    assert_eq!(int_array.min(), Some(1.to_variant()));
    assert_eq!(int_array.max(), Some(2.to_variant()));

    let uncomparable_array = Array::from(&[1.to_variant(), GodotString::from("two").to_variant()]);

    assert_eq!(uncomparable_array.min(), None);
    assert_eq!(uncomparable_array.max(), None);

    let empty_array = Array::new();

    assert_eq!(empty_array.min(), None);
    assert_eq!(empty_array.max(), None);
}

#[itest]
fn array_pick_random() {
    assert_eq!(Array::new().pick_random(), None);
    assert_eq!(Array::from(&[1]).pick_random(), Some(1.to_variant()));
}

#[itest]
fn array_set() {
    let mut array = Array::from(&[1, 2]);

    array.set(0, 3.to_variant());
    assert_eq!(array.get(0), 3.to_variant());

    expect_panic("Array index 2 out of bounds: length is 2", move || {
        array.set(2, 4.to_variant());
    });
}

#[itest]
fn array_push_pop() {
    let mut array = Array::from(&[1, 2]);

    array.push(3.to_variant());
    assert_eq!(array.pop(), Some(3.to_variant()));

    array.push_front(4.to_variant());
    assert_eq!(array.pop_front(), Some(4.to_variant()));

    assert_eq!(array.pop(), Some(2.to_variant()));
    assert_eq!(array.pop_front(), Some(1.to_variant()));

    assert_eq!(array.pop(), None);
    assert_eq!(array.pop_front(), None);
}

#[itest]
fn array_insert() {
    let mut array = Array::from(&[1, 2]);

    array.insert(0, 3.to_variant());
    assert_eq!(array.try_to_vec::<i64>().unwrap(), vec![3, 1, 2]);

    array.insert(3, 4.to_variant());
    assert_eq!(array.try_to_vec::<i64>().unwrap(), vec![3, 1, 2, 4]);
}

#[itest]
fn array_extend() {
    let mut array = Array::from(&[1, 2]);
    let other = Array::from(&[3, 4]);
    array.extend_array(other);
    assert_eq!(array.try_to_vec::<i64>().unwrap(), vec![1, 2, 3, 4]);
}

#[itest]
fn array_sort() {
    let mut array = Array::from(&[2, 1]);
    array.sort_unstable();
    assert_eq!(array.try_to_vec::<i64>().unwrap(), vec![1, 2]);
}

#[itest]
fn array_reverse() {
    let mut array = Array::from(&[1, 2]);
    array.reverse();
    assert_eq!(array.try_to_vec::<i64>().unwrap(), vec![2, 1]);
}

#[itest]
fn array_shuffle() {
    // Since the output is random, we just test that it doesn't crash.
    let mut array = Array::from(&[1i64]);
    array.shuffle();
    assert_eq!(array.try_to_vec::<i64>().unwrap(), vec![1]);
}
