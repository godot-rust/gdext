/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::prelude::*;

use crate::framework::{expect_panic, itest};

#[itest]
fn array_default() {
    assert_eq!(VariantArray::default().len(), 0);
}

#[itest]
fn array_new() {
    assert_eq!(VariantArray::new().len(), 0);
}

#[itest]
fn array_eq() {
    let a = array![1, 2];
    let b = array![1, 2];
    assert_eq!(a, b);

    let c = array![2, 1];
    assert_ne!(a, c);
}

#[itest]
fn typed_array_from_to_variant() {
    let array = array![1, 2];
    let variant = array.to_variant();
    let result = Array::try_from_variant(&variant).expect("typed array conversion should succeed");
    assert_eq!(result, array);
}

#[itest]
fn untyped_array_from_to_variant() {
    let array = varray![1, 2];
    let variant = array.to_variant();
    let result =
        VariantArray::try_from_variant(&variant).expect("untyped array conversion should succeed");
    assert_eq!(result, array);
}

#[itest]
fn array_from_packed_array() {
    let packed_array = PackedInt32Array::from(&[42]);
    let mut array = VariantArray::from(&packed_array);

    // This tests that the resulting array doesn't secretly have a runtime type assigned to it,
    // which is not reflected in our static type. It would make sense if it did, but Godot decided
    // otherwise: we get an untyped array.
    array.push(&GString::from("hi").to_variant());
    assert_eq!(array, varray![42, "hi"]);
}

#[itest]
fn array_from_iterator() {
    let array = Array::from_iter([1, 2]);

    assert_eq!(array.len(), 2);
    assert_eq!(array.at(0), 1);
    assert_eq!(array.at(1), 2);
}

#[itest]
fn array_from_slice() {
    let array = Array::from(&[1, 2]);

    assert_eq!(array.len(), 2);
    assert_eq!(array.at(0), 1);
    assert_eq!(array.at(1), 2);
}

#[itest]
fn array_try_into_vec() {
    let array = array![1, 2];

    #[allow(clippy::unnecessary_fallible_conversions)]
    let result = Vec::<i64>::try_from(&array);
    assert_eq!(result, Ok(vec![1, 2]));
}

#[itest]
fn array_iter_shared() {
    let array = array![1, 2];
    let mut iter = array.iter_shared();
    assert_eq!(iter.size_hint(), (2, Some(2)));
    assert_eq!(iter.next(), Some(1));
    assert_eq!(iter.size_hint(), (1, Some(1)));
    assert_eq!(iter.next(), Some(2));
    assert_eq!(iter.size_hint(), (0, Some(0)));
    assert_eq!(iter.next(), None);
}

#[itest]
fn array_hash() {
    let array = array![1, 2];
    // Just testing that it converts successfully from i64 to u32.
    array.hash();
}

#[itest]
fn array_share() {
    let mut array = array![1, 2];
    let shared = array.clone();
    array.set(0, 3);
    assert_eq!(shared.at(0), 3);
}

#[itest]
fn array_duplicate_shallow() {
    let subarray = array![2, 3];
    let array = varray![1, subarray];
    let duplicate = array.duplicate_shallow();
    Array::<i64>::try_from_variant(&duplicate.at(1))
        .unwrap()
        .set(0, 4);
    assert_eq!(subarray.at(0), 4);
}

#[itest]
fn array_duplicate_deep() {
    let subarray = array![2, 3];
    let array = varray![1, subarray];
    let duplicate = array.duplicate_deep();
    Array::<i64>::try_from_variant(&duplicate.at(1))
        .unwrap()
        .set(0, 4);
    assert_eq!(subarray.at(0), 2);
}

#[itest]
fn array_subarray_shallow() {
    let array = array![0, 1, 2, 3, 4, 5];
    let slice = array.subarray_shallow(5, 1, Some(-2));
    assert_eq!(slice, array![5, 3]);

    let subarray = array![2, 3];
    let array = varray![1, subarray];
    let slice = array.subarray_shallow(1, 2, None);
    Array::<i64>::try_from_variant(&slice.at(0))
        .unwrap()
        .set(0, 4);
    assert_eq!(subarray.at(0), 4);
}

#[itest]
fn array_subarray_deep() {
    let array = array![0, 1, 2, 3, 4, 5];
    let slice = array.subarray_deep(5, 1, Some(-2));
    assert_eq!(slice, array![5, 3]);

    let subarray = array![2, 3];
    let array = varray![1, subarray];
    let slice = array.subarray_deep(1, 2, None);
    Array::<i64>::try_from_variant(&slice.at(0))
        .unwrap()
        .set(0, 4);
    assert_eq!(subarray.at(0), 2);
}

#[itest]
fn array_get() {
    let array = array![1, 2];

    assert_eq!(array.at(0), 1);
    assert_eq!(array.at(1), 2);
    expect_panic("Array index 2 out of bounds: length is 2", || {
        array.at(2);
    });
}

#[itest]
fn array_try_get() {
    let array = array![1, 2];

    assert_eq!(array.get(0), Some(1));
    assert_eq!(array.get(1), Some(2));
    assert_eq!(array.get(2), None);
}

#[itest]
fn array_first_last() {
    let array = array![1, 2];

    assert_eq!(array.front(), Some(1));
    assert_eq!(array.back(), Some(2));

    let empty_array = VariantArray::new();

    assert_eq!(empty_array.front(), None);
    assert_eq!(empty_array.back(), None);
}

#[itest]
fn array_find() {
    let array = array![1, 2, 1];

    assert_eq!(array.find(0, None), None);
    assert_eq!(array.find(1, None), Some(0));
    assert_eq!(array.find(1, Some(1)), Some(2));
}

#[itest]
fn array_rfind() {
    let array = array![1, 2, 1];

    assert_eq!(array.rfind(0, None), None);
    assert_eq!(array.rfind(1, None), Some(2));
    assert_eq!(array.rfind(1, Some(1)), Some(0));
}

#[itest]
fn array_min_max() {
    let int_array = array![1, 2];

    assert_eq!(int_array.min(), Some(1));
    assert_eq!(int_array.max(), Some(2));

    let uncomparable_array = varray![1, GString::from("two")];

    assert_eq!(uncomparable_array.min(), None);
    assert_eq!(uncomparable_array.max(), None);

    let empty_array = VariantArray::new();

    assert_eq!(empty_array.min(), None);
    assert_eq!(empty_array.max(), None);
}

#[itest]
fn array_pick_random() {
    assert_eq!(VariantArray::new().pick_random(), None);
    assert_eq!(array![1].pick_random(), Some(1));
}

#[itest]
fn array_set() {
    let mut array = array![1, 2];

    array.set(0, 3);
    assert_eq!(array.at(0), 3);

    expect_panic("Array index 2 out of bounds: length is 2", move || {
        array.set(2, 4);
    });
}

#[itest]
fn array_set_readonly() {
    let mut array = array![1, 2].into_read_only();

    #[cfg(debug_assertions)]
    expect_panic("Mutating read-only array in Debug mode", || {
        array.set(0, 3);
    });

    #[cfg(not(debug_assertions))]
    array.set(0, 3); // silently fails.

    assert_eq!(array.at(0), 1);
}

#[itest]
fn array_push_pop() {
    let mut array = array![1, 2];

    array.push(3);
    assert_eq!(array.pop(), Some(3));

    array.push_front(4);
    assert_eq!(array.pop_front(), Some(4));

    assert_eq!(array.pop(), Some(2));
    assert_eq!(array.pop_front(), Some(1));

    assert_eq!(array.pop(), None);
    assert_eq!(array.pop_front(), None);
}

#[itest]
fn array_insert() {
    let mut array = array![1, 2];

    array.insert(0, 3);
    assert_eq!(array, array![3, 1, 2]);

    array.insert(3, 4);
    assert_eq!(array, array![3, 1, 2, 4]);
}

#[itest]
fn array_extend() {
    let mut array = array![1, 2];
    let other = array![3, 4];
    array.extend_array(&other);
    assert_eq!(array, array![1, 2, 3, 4]);
}

#[itest]
fn array_reverse() {
    let mut array = array![1, 2];
    array.reverse();
    assert_eq!(array, array![2, 1]);
}

#[itest]
fn array_shuffle() {
    let mut array = array![1];
    array.shuffle();
    assert_eq!(array, array![1]);
}

#[itest]
fn array_mixed_values() {
    let int = 1;
    let string = GString::from("hello");
    let packed_array = PackedByteArray::from(&[1, 2]);
    let typed_array = array![1, 2];
    let object = Object::new_alloc();
    let node = Node::new_alloc();
    let engine_refc = RefCounted::new_gd();
    let user_refc = ArrayTest::new_gd(); // user RefCounted type

    let array = varray![
        int,
        string,
        packed_array,
        typed_array,
        object,
        node,
        engine_refc,
        user_refc,
    ];

    assert_eq!(i64::try_from_variant(&array.at(0)).unwrap(), int);
    assert_eq!(GString::try_from_variant(&array.at(1)).unwrap(), string);
    assert_eq!(
        PackedByteArray::try_from_variant(&array.at(2)).unwrap(),
        packed_array
    );
    assert_eq!(Array::try_from_variant(&array.at(3)).unwrap(), typed_array);
    assert_eq!(
        Gd::<Object>::try_from_variant(&array.at(4))
            .unwrap()
            .instance_id(),
        object.instance_id()
    );
    assert_eq!(
        Gd::<Node>::try_from_variant(&array.at(5))
            .unwrap()
            .instance_id(),
        node.instance_id()
    );

    assert_eq!(
        Gd::<RefCounted>::try_from_variant(&array.at(6))
            .unwrap()
            .instance_id(),
        engine_refc.instance_id()
    );
    assert_eq!(
        Gd::<ArrayTest>::try_from_variant(&array.at(7))
            .unwrap()
            .instance_id(),
        user_refc.instance_id()
    );

    object.free();
    node.free();
}

#[itest]
fn untyped_array_pass_to_godot_func() {
    let mut node = Node::new_alloc();
    node.queue_free(); // Do not leak even if the test fails.

    assert_eq!(
        node.callv("has_signal", &varray!["tree_entered"]),
        true.to_variant()
    );
}

#[itest]
fn untyped_array_return_from_godot_func() {
    // There aren't many API functions that return an untyped array.
    let mut node = Node::new_alloc();
    let mut child = Node::new_alloc();
    child.set_name("child_node");
    node.add_child(&child);
    node.queue_free(); // Do not leak even if the test fails.
    let result = node.get_node_and_resource("child_node");

    assert_eq!(result, varray![child, Variant::nil(), NodePath::default()]);
}

// Conditional, so we don't need Texture2DArray > ImageTextureLayered > TextureLayered > Texture in minimal codegen.
// Potential alternatives (search for "typedarray::" in extension_api.json):
// - ClassDB::class_get_signal_list() -> Array<Dictionary>
// - Compositor::set_compositor_effects( Array<Gd<Compositor>> )
#[cfg(feature = "codegen-full-experimental")]
#[itest]
fn typed_array_pass_to_godot_func() {
    use godot::classes::image::Format;
    use godot::classes::{Image, Texture2DArray};
    use godot::global::Error;

    let mut image = Image::new_gd();
    image.set_data(
        2,
        4,
        false,
        Format::L8,
        &PackedByteArray::from(&[255, 0, 255, 0, 0, 255, 0, 255]),
    );
    let images = array![&image];
    let mut texture = Texture2DArray::new_gd();
    let error = texture.create_from_images(&images);

    assert_eq!(error, Error::OK);
    assert_eq!((texture.get_width(), texture.get_height()), (2, 4));
}

#[itest]
fn typed_array_return_from_godot_func() {
    let mut node = Node::new_alloc();
    let mut child = Node::new_alloc();
    child.set_name("child_node");
    node.add_child(&child);
    node.queue_free(); // Do not leak even if the test fails.
    let children = node.get_children();

    assert_eq!(children, array![&child]);
}

#[itest]
fn typed_array_try_from_untyped() {
    let node = Node::new_alloc();
    let array = VariantArray::from(&[node.clone().to_variant()]);

    array
        .to_variant()
        .try_to::<Array<Option<Gd<Node>>>>()
        .expect_err("untyped array should not coerce to typed array");

    node.free();
}

#[itest]
fn untyped_array_try_from_typed() {
    let node = Node::new_alloc();
    let array = Array::<Option<Gd<Node>>>::from(&[Some(node.clone())]);

    array
        .to_variant()
        .try_to::<VariantArray>()
        .expect_err("typed array should not coerce to untyped array");

    node.free();
}

#[itest]
fn array_should_format_with_display() {
    let a = array![1, 2, 3, 4];
    assert_eq!(format!("{a}"), "[1, 2, 3, 4]");

    let a = Array::<real>::new();
    assert_eq!(format!("{a}"), "[]");
}

#[itest]
fn array_sort_unstable() {
    let mut array = array![2, 1];
    array.sort_unstable();
    assert_eq!(array, array![1, 2]);
}

#[itest]
#[cfg(since_api = "4.2")]
fn array_sort_unstable_by() {
    let mut array: Array<i32> = array![2, 1, 4, 3];
    array.sort_unstable_by(|a, b| a.cmp(b));
    assert_eq!(array, array![1, 2, 3, 4]);
}

#[itest]
#[cfg(since_api = "4.2")]
fn array_sort_unstable_custom() {
    let mut a = array![1, 2, 3, 4];
    let func = backwards_sort_callable();
    a.sort_unstable_custom(&func);
    assert_eq!(a, array![4, 3, 2, 1]);
}

#[itest]
fn array_bsearch() {
    let array = array![1, 3];

    assert_eq!(array.bsearch(0), 0);
    assert_eq!(array.bsearch(1), 0);
    assert_eq!(array.bsearch(2), 1);
    assert_eq!(array.bsearch(3), 1);
    assert_eq!(array.bsearch(4), 2);
}

#[itest]
#[cfg(since_api = "4.2")]
fn array_bsearch_by() {
    let a: Array<i32> = array![1, 2, 4, 5];

    assert_eq!(a.bsearch_by(|e| e.cmp(&2)), Ok(1));
    assert_eq!(a.bsearch_by(|e| e.cmp(&4)), Ok(2));

    assert_eq!(a.bsearch_by(|e| e.cmp(&0)), Err(0));
    assert_eq!(a.bsearch_by(|e| e.cmp(&3)), Err(2));
    assert_eq!(a.bsearch_by(|e| e.cmp(&9)), Err(4));
}

#[itest]
#[cfg(since_api = "4.2")]
fn array_bsearch_custom() {
    let a = array![5, 4, 2, 1];
    let func = backwards_sort_callable();
    assert_eq!(a.bsearch_custom(1, &func), 3);
    assert_eq!(a.bsearch_custom(3, &func), 2);
}

#[cfg(since_api = "4.2")]
fn backwards_sort_callable() -> Callable {
    Callable::from_local_fn("sort backwards", |args: &[&Variant]| {
        let res = args[0].to::<i32>() > args[1].to::<i32>();
        Ok(res.to_variant())
    })
}

#[itest]
fn array_shrink() {
    let mut a = array![1, 5, 4, 3, 8];

    assert!(!a.shrink(10));
    assert_eq!(a.len(), 5);

    assert!(a.shrink(3));
    assert_eq!(a.len(), 3);
    assert_eq!(a, array![1, 5, 4]);
}

#[itest]
fn array_resize() {
    let mut a = array!["hello", "bar", "mixed", "baz", "meow"];

    let new = GString::from("new!");

    a.resize(10, &new);
    assert_eq!(a.len(), 10);
    assert_eq!(
        a,
        array!["hello", "bar", "mixed", "baz", "meow", &new, &new, &new, &new, &new]
    );

    a.resize(2, &new);
    assert_eq!(a, array!["hello", "bar"]);

    a.resize(0, &new);
    assert_eq!(a, Array::new());
}

#[derive(GodotClass, Debug)]
#[class(init, base=RefCounted)]
struct ArrayTest;

#[godot_api]
impl ArrayTest {
    #[func]
    fn return_typed_array(&self, n: i64) -> Array<i64> {
        (1..(n + 1)).collect()
    }
}
