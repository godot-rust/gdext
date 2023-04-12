/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::{expect_panic, itest};
use godot::prelude::*;

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
    let result = Array::try_from_variant(&variant);
    assert_eq!(result, Ok(array));
}

#[itest]
fn untyped_array_from_to_variant() {
    let array = varray![1, 2];
    let variant = array.to_variant();
    let result = VariantArray::try_from_variant(&variant);
    assert_eq!(result, Ok(array));
}

#[itest]
fn array_from_packed_array() {
    let packed_array = PackedInt32Array::from(&[42]);
    let mut array = VariantArray::from(&packed_array);
    // This tests that the resulting array doesn't secretly have a runtime type assigned to it,
    // which is not reflected in our static type. It would make sense if it did, but Godot decided
    // otherwise: we get an untyped array.
    array.push(GodotString::from("hi").to_variant());
    assert_eq!(array, varray![42, "hi"]);
}

#[itest]
fn array_from_iterator() {
    let array = Array::from_iter([1, 2]);

    assert_eq!(array.len(), 2);
    assert_eq!(array.get(0), 1);
    assert_eq!(array.get(1), 2);
}

#[itest]
fn array_from_slice() {
    let array = Array::from(&[1, 2]);

    assert_eq!(array.len(), 2);
    assert_eq!(array.get(0), 1);
    assert_eq!(array.get(1), 2);
}

#[itest]
fn array_try_into_vec() {
    let array = array![1, 2];
    let result = Vec::<i64>::try_from(&array);
    assert_eq!(result, Ok(vec![1, 2]));
}

#[itest]
fn array_iter_shared() {
    let array = array![1, 2];
    let mut iter = array.iter_shared();
    assert_eq!(iter.next(), Some(1));
    assert_eq!(iter.next(), Some(2));
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
    let shared = array.share();
    array.set(0, 3);
    assert_eq!(shared.get(0), 3);
}

#[itest]
fn array_duplicate_shallow() {
    let subarray = array![2, 3];
    let array = varray![1, subarray];
    let duplicate = array.duplicate_shallow();
    Array::<i64>::try_from_variant(&duplicate.get(1))
        .unwrap()
        .set(0, 4);
    assert_eq!(subarray.get(0), 4);
}

#[itest]
fn array_duplicate_deep() {
    let subarray = array![2, 3];
    let array = varray![1, subarray];
    let duplicate = array.duplicate_deep();
    Array::<i64>::try_from_variant(&duplicate.get(1))
        .unwrap()
        .set(0, 4);
    assert_eq!(subarray.get(0), 2);
}

#[itest]
fn array_slice_shallow() {
    let array = array![0, 1, 2, 3, 4, 5];
    let slice = array.slice_shallow(5, 1, Some(-2));
    assert_eq!(slice, array![5, 3]);

    let subarray = array![2, 3];
    let array = varray![1, subarray];
    let slice = array.slice_shallow(1, 2, None);
    Array::<i64>::try_from_variant(&slice.get(0))
        .unwrap()
        .set(0, 4);
    assert_eq!(subarray.get(0), 4);
}

#[itest]
fn array_slice_deep() {
    let array = array![0, 1, 2, 3, 4, 5];
    let slice = array.slice_deep(5, 1, Some(-2));
    assert_eq!(slice, array![5, 3]);

    let subarray = array![2, 3];
    let array = varray![1, subarray];
    let slice = array.slice_deep(1, 2, None);
    Array::<i64>::try_from_variant(&slice.get(0))
        .unwrap()
        .set(0, 4);
    assert_eq!(subarray.get(0), 2);
}

#[itest]
fn array_get() {
    let array = array![1, 2];

    assert_eq!(array.get(0), 1);
    assert_eq!(array.get(1), 2);
    expect_panic("Array index 2 out of bounds: length is 2", || {
        array.get(2);
    });
}

#[itest]
fn array_first_last() {
    let array = array![1, 2];

    assert_eq!(array.first(), Some(1));
    assert_eq!(array.last(), Some(2));

    let empty_array = VariantArray::new();

    assert_eq!(empty_array.first(), None);
    assert_eq!(empty_array.last(), None);
}

#[itest]
fn array_binary_search() {
    let array = array![1, 3];

    assert_eq!(array.binary_search(&0), 0);
    assert_eq!(array.binary_search(&1), 0);
    assert_eq!(array.binary_search(&2), 1);
    assert_eq!(array.binary_search(&3), 1);
    assert_eq!(array.binary_search(&4), 2);
}

#[itest]
fn array_find() {
    let array = array![1, 2, 1];

    assert_eq!(array.find(&0, None), None);
    assert_eq!(array.find(&1, None), Some(0));
    assert_eq!(array.find(&1, Some(1)), Some(2));
}

#[itest]
fn array_rfind() {
    let array = array![1, 2, 1];

    assert_eq!(array.rfind(&0, None), None);
    assert_eq!(array.rfind(&1, None), Some(2));
    assert_eq!(array.rfind(&1, Some(1)), Some(0));
}

#[itest]
fn array_min_max() {
    let int_array = array![1, 2];

    assert_eq!(int_array.min(), Some(1));
    assert_eq!(int_array.max(), Some(2));

    let uncomparable_array = varray![1, GodotString::from("two")];

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
    assert_eq!(array.get(0), 3);

    expect_panic("Array index 2 out of bounds: length is 2", move || {
        array.set(2, 4);
    });
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
    array.extend_array(other);
    assert_eq!(array, array![1, 2, 3, 4]);
}

#[itest]
fn array_sort() {
    let mut array = array![2, 1];
    array.sort_unstable();
    assert_eq!(array, array![1, 2]);
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
    let string = GodotString::from("hello");
    let packed_array = PackedByteArray::from(&[1, 2]);
    let typed_array = array![1, 2];
    let object = Object::new_alloc();
    let node = Node::new_alloc();
    let ref_counted = RefCounted::new();

    let array = varray![
        int,
        string,
        packed_array,
        typed_array,
        object,
        node,
        ref_counted,
    ];

    assert_eq!(i64::try_from_variant(&array.get(0)).unwrap(), int);
    assert_eq!(
        GodotString::try_from_variant(&array.get(1)).unwrap(),
        string
    );
    assert_eq!(
        PackedByteArray::try_from_variant(&array.get(2)).unwrap(),
        packed_array
    );
    assert_eq!(Array::try_from_variant(&array.get(3)).unwrap(), typed_array);
    assert_eq!(
        Gd::<Object>::try_from_variant(&array.get(4))
            .unwrap()
            .instance_id(),
        object.instance_id()
    );
    assert_eq!(
        Gd::<Node>::try_from_variant(&array.get(5))
            .unwrap()
            .instance_id(),
        node.instance_id()
    );
    assert_eq!(
        Gd::<RefCounted>::try_from_variant(&array.get(6))
            .unwrap()
            .instance_id(),
        ref_counted.instance_id()
    );

    object.free();
    node.free();
}

#[itest]
fn untyped_array_pass_to_godot_func() {
    let mut node = Node::new_alloc();
    node.queue_free(); // Do not leak even if the test fails.

    assert_eq!(
        node.callv(StringName::from("has_signal"), varray!["tree_entered"]),
        true.to_variant()
    );
}

#[itest]
fn untyped_array_return_from_godot_func() {
    use godot::engine::node::InternalMode;
    use godot::engine::Node;

    // There aren't many API functions that return an untyped array.
    let mut node = Node::new_alloc();
    let mut child = Node::new_alloc();
    child.set_name("child_node".into());
    node.add_child(child.share(), false, InternalMode::INTERNAL_MODE_DISABLED);
    node.queue_free(); // Do not leak even if the test fails.
    let result = node.get_node_and_resource("child_node".into());

    assert_eq!(result, varray![child, Variant::nil(), NodePath::default()]);
}

// TODO All API functions that take a `Array` are even more obscure and not included in
// `SELECTED_CLASSES`. Decide if this test is worth having `Texture2DArray` and `Image` and their
// ancestors in the list.
#[itest]
fn typed_array_pass_to_godot_func() {
    use godot::engine::global::Error;
    use godot::engine::image::Format;
    use godot::engine::{Image, Texture2DArray};

    let mut image = Image::new();
    image.set_data(
        2,
        4,
        false,
        Format::FORMAT_L8,
        PackedByteArray::from(&[255, 0, 255, 0, 0, 255, 0, 255]),
    );
    let images = array![image];
    let mut texture = Texture2DArray::new();
    let error = texture.create_from_images(images);

    assert_eq!(error, Error::OK);
    assert_eq!((texture.get_width(), texture.get_height()), (2, 4));
}

#[itest]
fn typed_array_return_from_godot_func() {
    use godot::engine::node::InternalMode;
    use godot::engine::Node;

    let mut node = Node::new_alloc();
    let mut child = Node::new_alloc();
    child.set_name("child_node".into());
    node.add_child(child.share(), false, InternalMode::INTERNAL_MODE_DISABLED);
    node.queue_free(); // Do not leak even if the test fails.
    let children = node.get_children(false);

    assert_eq!(children, array![child]);
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
