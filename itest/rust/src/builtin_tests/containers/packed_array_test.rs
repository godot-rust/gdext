/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::builtin::{
    vdict, Color, GString, PackedArray, PackedByteArray, PackedColorArray, PackedFloat32Array,
    PackedInt32Array, PackedStringArray, Variant, Vector3,
};
use godot::global::godot_str;
use godot::meta::{owned_into_arg, ref_to_arg};
use godot::prelude::ToGodot;

use crate::framework::{expect_panic, itest};

#[itest]
fn packed_array_default() {
    assert_eq!(PackedByteArray::default().len(), 0);
}

#[itest]
fn packed_array_new() {
    assert_eq!(PackedByteArray::new().len(), 0);
}

#[itest]
fn packed_array_from_iterator() {
    let array = PackedByteArray::from_iter([1, 2]);

    assert_eq!(array.len(), 2);
    assert_eq!(array[0], 1);
    assert_eq!(array[1], 2);
}

#[itest]
fn packed_array_from_vec_str() {
    let string_array = PackedStringArray::from(vec!["hello".into(), "world".into()]);

    assert_eq!(string_array.len(), 2);
    assert_eq!(string_array[0], GString::from("hello"));
    assert_eq!(string_array[1], GString::from("world"));
}

#[itest]
fn packed_array_from_vec_i32() {
    let int32_array = PackedInt32Array::from(vec![1, 2]);

    assert_eq!(int32_array.len(), 2);
    assert_eq!(int32_array[0], 1);
    assert_eq!(int32_array[1], 2);
}

#[itest]
fn packed_array_from_vec_color() {
    const SRC: [Color; 3] = [
        Color::from_rgb(1., 0., 0.),
        Color::from_rgb(0., 1., 0.),
        Color::from_rgb(0., 0., 1.),
    ];
    let color_array = PackedColorArray::from(Vec::from(SRC));

    assert_eq!(color_array.len(), SRC.len());
    for (i, c) in SRC.into_iter().enumerate() {
        assert_eq!(color_array[i], c, "value mismatch at index {i}");
    }
}

#[itest]
fn packed_array_from_array_str() {
    let string_array = PackedStringArray::from(["hello".into(), "world".into()]);

    assert_eq!(string_array.len(), 2);
    assert_eq!(string_array[0], GString::from("hello"));
    assert_eq!(string_array[1], GString::from("world"));
}

#[itest]
fn packed_array_from_array_i32() {
    let int32_array = PackedInt32Array::from([1, 2]);

    assert_eq!(int32_array.len(), 2);
    assert_eq!(int32_array[0], 1);
    assert_eq!(int32_array[1], 2);
}

#[itest]
fn packed_array_from_array_color() {
    const SRC: [Color; 3] = [
        Color::from_rgb(1., 0., 0.),
        Color::from_rgb(0., 1., 0.),
        Color::from_rgb(0., 0., 1.),
    ];
    let color_array = PackedColorArray::from(SRC);

    assert_eq!(color_array.len(), SRC.len());
    for (i, c) in SRC.into_iter().enumerate() {
        assert_eq!(color_array[i], c, "value mismatch at index {i}");
    }
}

#[itest]
fn packed_array_to_vec() {
    let array = PackedByteArray::new();
    assert_eq!(array.to_vec(), Vec::<u8>::new());
    let array = PackedByteArray::from(&[1, 2]);
    assert_eq!(array.to_vec(), vec![1, 2]);
}

/*
#[itest(skip)]
fn packed_array_into_iterator() {
    let array = PackedByteArray::from(&[1, 2]);
    let mut iter = array.into_iter();
    assert_eq!(iter.next(), Some(1));
    assert_eq!(iter.next(), Some(2));
    assert_eq!(iter.next(), None);
}
*/

#[itest]
fn packed_array_eq() {
    assert_eq!(
        PackedByteArray::from(&[1, 2]),
        PackedByteArray::from(&[1, 2])
    );
    assert_ne!(
        PackedByteArray::from(&[1, 2]),
        PackedByteArray::from(&[1, 1])
    );
    assert_ne!(
        PackedFloat32Array::from(&[f32::NAN]),
        PackedFloat32Array::from(&[f32::NAN])
    );
}

#[itest]
fn packed_array_clone() {
    let mut array = PackedByteArray::from(&[1, 2]);
    #[allow(clippy::redundant_clone)]
    let clone = array.clone();
    array[0] = 3;

    assert_eq!(clone[0], 1);
}

#[itest]
fn packed_array_subarray() {
    let array = PackedByteArray::from(&[1, 2, 3]);
    let subarray = array.subarray(1, 2);

    assert_eq!(subarray.to_vec(), vec![2]);
}

#[itest]
fn packed_array_as_slice() {
    let a = PackedByteArray::from(&[1, 2, 3]);
    #[allow(clippy::redundant_clone)]
    let b = a.clone();

    let slice_a = a.as_slice();
    let slice_b = b.as_slice();

    assert_eq!(slice_a, &[1, 2, 3]);
    assert_eq!(slice_a, slice_b);
    assert_eq!(
        slice_a.as_ptr(),
        slice_b.as_ptr(),
        "copy-on-write without modification returns aliased slice"
    );

    let empty = PackedStringArray::new();
    assert_eq!(empty.as_slice(), &[]);
}

#[itest]
fn packed_array_as_mut_slice() {
    let a = PackedByteArray::from(&[1, 2, 3]);
    let mut b = a.clone();

    let slice_a = a.as_slice();
    let slice_b = b.as_mut_slice(); // triggers CoW

    assert_eq!(slice_a, &mut [1, 2, 3]);
    assert_eq!(slice_a, slice_b);
    assert_ne!(
        slice_a.as_ptr(),
        slice_b.as_ptr(),
        "copy-on-write with modification must return independent slice"
    );

    let mut empty = PackedStringArray::new();
    assert_eq!(empty.as_mut_slice(), &mut []);
}

#[itest]
fn packed_array_index() {
    let array = PackedByteArray::from(&[1, 2]);

    assert_eq!(array[0], 1);
    assert_eq!(array[1], 2);
    expect_panic("Array index 2 out of bounds: length is 2", || {
        let _ = array[2];
    });

    let mut array = PackedStringArray::new();
    expect_panic("Array index 0 out of bounds: length is 0", || {
        let _ = array[0];
    });

    // Note: push works on &str as well as GString, or references of it.
    array.push("first");
    array.push(&GString::from("second"));

    assert_eq!(array[0], "first".into());
    assert_eq!(array[1], "second".into());

    array[0] = GString::from("begin");
    assert_eq!(array[0], "begin".into());
}

#[itest]
fn packed_array_get() {
    let array = PackedByteArray::from(&[1, 2]);

    assert_eq!(array.get(0), Some(1));
    assert_eq!(array.get(1), Some(2));
    assert_eq!(array.get(2), None);
}

#[itest]
fn packed_array_binary_search() {
    let array = PackedByteArray::from(&[1, 3]);

    assert_eq!(array.bsearch(0), 0);
    assert_eq!(array.bsearch(1), 0);
    assert_eq!(array.bsearch(2), 1);
    assert_eq!(array.bsearch(3), 1);
    assert_eq!(array.bsearch(4), 2);
}

#[itest]
fn packed_array_find() {
    let array = PackedByteArray::from(&[1, 2, 1]);

    assert_eq!(array.find(0, None), None);
    assert_eq!(array.find(1, None), Some(0));
    assert_eq!(array.find(1, Some(1)), Some(2));
}

#[itest]
fn packed_array_rfind() {
    let array = PackedByteArray::from(&[1, 2, 1]);

    assert_eq!(array.rfind(0, None), None);
    assert_eq!(array.rfind(1, None), Some(2));
    assert_eq!(array.rfind(1, Some(1)), Some(0));
}

#[itest]
fn packed_array_push() {
    let mut array = PackedByteArray::from(&[1, 2]);

    array.push(3);

    assert_eq!(array.len(), 3);
    assert_eq!(array[2], 3);
}

#[itest]
fn packed_array_insert() {
    let mut array = PackedByteArray::from(&[1, 2]);

    array.insert(0, 3);
    assert_eq!(array.to_vec(), vec![3, 1, 2]);

    array.insert(3, 4);
    assert_eq!(array.to_vec(), vec![3, 1, 2, 4]);
}

fn test_extend<F, I>(make_iter: F)
where
    F: Fn(i32) -> I,
    I: Iterator<Item = i32>,
{
    // The logic in `extend()` is not trivial, so we test it for a wide range of sizes: powers of two, also plus and minus one.
    // This includes zero. We go up to 2^12, which is 4096, because the internal buffer is currently 2048 bytes (512 `i32`s)
    // and we want to be future-proof in case it's ever enlarged.
    let lengths = (0..12i32)
        .flat_map(|i| {
            let b = 1 << i;
            [b - 1, b, b + 1]
        })
        .collect::<Vec<_>>();

    for &len_a in &lengths {
        for &len_b in &lengths {
            let iter = make_iter(len_b);
            let mut array = PackedInt32Array::from_iter(0..len_a);
            array.extend(iter);
            let expected = (0..len_a).chain(0..len_b).collect::<Vec<_>>();
            assert_eq!(array.to_vec(), expected, "len_a = {len_a}, len_b = {len_b}",);
        }
    }
}

#[itest]
fn packed_array_extend_known_size() {
    // Create an iterator whose `size_hint()` returns `(len, Some(len))`.
    test_extend(|len| 0..len);
}

#[itest]
fn packed_array_extend_unknown_size() {
    // Create an iterator whose `size_hint()` returns `(0, None)`.
    test_extend(|len| {
        let mut item = 0;
        std::iter::from_fn(move || {
            let result = if item < len { Some(item) } else { None };
            item += 1;
            result
        })
    });
}

#[itest]
fn packed_array_extend_array() {
    let mut array = PackedByteArray::from(&[1, 2]);
    let other = PackedByteArray::from(&[3, 4]);
    array.extend_array(&other);
    assert_eq!(array.to_vec(), vec![1, 2, 3, 4]);
}

#[itest]
fn packed_array_sort() {
    let mut array = PackedByteArray::from(&[2, 1]);
    array.sort();
    assert_eq!(array.to_vec(), vec![1, 2]);
}

#[itest]
fn packed_array_reverse() {
    let mut array = PackedByteArray::from(&[1, 2]);
    array.reverse();
    assert_eq!(array.to_vec(), vec![2, 1]);
}

#[itest]
fn packed_array_format() {
    let a = PackedByteArray::from(&[2, 1]);
    assert_eq!(format!("{a}"), "[2, 1]");

    let a = PackedByteArray::new();
    assert_eq!(format!("{a}"), "[]");
}

#[itest]
fn packed_byte_array_encode_decode() {
    let a = PackedByteArray::from(&[0xAB, 0xCD, 0x12]);

    assert_eq!(a.decode_u8(0), Ok(0xAB));
    assert_eq!(a.decode_u8(2), Ok(0x12));
    assert_eq!(a.decode_u16(1), Ok(0x12CD)); // Currently little endian, but this may change.
    assert_eq!(a.decode_u16(2), Err(()));
    assert_eq!(a.decode_u32(0), Err(()));

    let mut a = a;
    a.encode_u16(1, 0xEF34).unwrap();
    assert_eq!(a.decode_u8(0), Ok(0xAB));
    assert_eq!(a.decode_u8(1), Ok(0x34));
    assert_eq!(a.decode_u8(2), Ok(0xEF));
}

#[itest]
fn packed_byte_array_encode_decode_variant() {
    let variant = vdict! {
        "s": "some string",
        "i": -12345,
    }
    .to_variant();

    let mut a = PackedByteArray::new();
    a.resize(40);

    // NIL is a valid, encodable value.
    let nil = a.encode_var(3, &Variant::nil(), false);
    assert_eq!(nil, Ok(4));

    let bytes = a.encode_var(3, &variant, false);
    assert_eq!(bytes, Err(()));

    a.resize(80);
    let bytes = a.encode_var(3, &variant, false);
    assert_eq!(bytes, Ok(60)); // Size may change; in that case we only need to verify is_ok().

    // Decoding. Detects garbage.
    let decoded = a.decode_var(3, false).expect("decode_var() succeeds");
    assert_eq!(decoded.0, variant);
    assert_eq!(decoded.1, 60);

    let decoded = a.decode_var(4, false);
    assert_eq!(decoded, Err(()));

    // Decoding with NILs.
    let decoded = a.decode_var_allow_nil(3, false);
    assert_eq!(decoded.0, variant);
    assert_eq!(decoded.1, 60);

    // Interprets garbage as NIL Variant with size 4.
    let decoded = a.decode_var_allow_nil(4, false);
    assert_eq!(decoded.0, Variant::nil());
    assert_eq!(decoded.1, 4);

    // Even last 4 bytes (still zeroed memory) is allegedly a variant.
    let decoded = a.decode_var_allow_nil(76, false);
    assert_eq!(decoded.0, Variant::nil());
    assert_eq!(decoded.1, 4);

    // Only running out of size "fails".
    let decoded = a.decode_var_allow_nil(77, false);
    assert_eq!(decoded.0, Variant::nil());
    assert_eq!(decoded.1, 0);
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Generic PackedArray<T> tests

#[itest]
fn packed_array_generic_extend() {
    test_generic_extend(|n| (0..n as u8).collect());
    test_generic_extend(|n| (0..n as i32).collect());
    test_generic_extend(|n| {
        (0..n)
            .map(|i| GString::from(format!("test_{}", i)))
            .collect()
    });
    test_generic_extend(|n| {
        (0..n)
            .map(|i| Vector3::new(i as f32, (i * 2) as f32, (i * 3) as f32))
            .collect()
    });
}

#[itest]
fn packed_array_generic_push() {
    test_generic_push(|n| (0..n as u8).collect());
    test_generic_push(|n| (0..n as i32).collect());
    test_generic_push(|n| {
        (0..n)
            .map(|i| GString::from(format!("item_{}", i)))
            .collect()
    });
    test_generic_push(|n| {
        (0..n)
            .map(|i| Vector3::new(i as f32, (i + 10) as f32, (i + 20) as f32))
            .collect()
    });
}

#[itest]
fn packed_array_generic_insert_remove() {
    test_generic_insert_remove(|_| vec![10u8, 20u8, 30u8]);
    test_generic_insert_remove(|_| vec![100i32, 200i32, 300i32]);
    test_generic_insert_remove(|_| {
        vec![
            GString::from("first"),
            GString::from("second"),
            GString::from("third"),
        ]
    });
    test_generic_insert_remove(|_| {
        vec![
            Vector3::new(1.0, 2.0, 3.0),
            Vector3::new(4.0, 5.0, 6.0),
            Vector3::new(7.0, 8.0, 9.0),
        ]
    });
}

#[itest]
fn packed_array_generic_extend_array() {
    test_generic_extend_array(|n| (0..n as u8).collect());
    test_generic_extend_array(|n| (0..n as i32).collect());
    test_generic_extend_array(|n| (0..n).map(|i| godot_str!("str_{}", i)).collect());
    test_generic_extend_array(|n| {
        (0..n)
            .map(|i| Vector3::new(i as f32, i as f32 * 2.0, i as f32 * 3.0))
            .collect()
    });
}

#[itest]
fn packed_array_generic_sort_reverse() {
    test_generic_sort_reverse(|| vec![3u8, 1u8, 4u8, 1u8, 5u8]);
    test_generic_sort_reverse(|| vec![42i32, 17i32, 99i32, 3i32]);
}

#[itest]
fn packed_array_generic_resize_fill() {
    test_generic_resize_fill(|| 42u8);
    test_generic_resize_fill(|| 999i32);
    test_generic_resize_fill(|| GString::from("filled"));
    test_generic_resize_fill(|| Vector3::new(10.0, 20.0, 30.0));
}

#[itest]
fn packed_array_generic_find_contains() {
    test_generic_find_contains(|| vec![5u8, 10u8, 15u8, 20u8]);
    test_generic_find_contains(|| vec![100i32, 200i32, 300i32]);
    test_generic_find_contains(|| {
        vec![
            GString::from("alpha"),
            GString::from("beta"),
            GString::from("gamma"),
        ]
    });
    test_generic_find_contains(|| {
        vec![
            Vector3::new(1.0, 0.0, 0.0),
            Vector3::new(0.0, 1.0, 0.0),
            Vector3::new(0.0, 0.0, 1.0),
        ]
    });
}

#[itest]
fn packed_array_generic_as_slice_as_mut_slice() {
    let mut array = PackedArray::<i32>::new();
    array.extend([10, 20, 30]);

    // Test as_slice
    let slice = array.as_slice();
    assert_eq!(slice, &[10, 20, 30]);

    // Test as_mut_slice
    let mut_slice = array.as_mut_slice();
    mut_slice[1] = 99;
    assert_eq!(array[1], 99);
    assert_eq!(array.as_slice(), &[10, 99, 30]);
}

#[itest]
fn packed_array_generic_subarray() {
    let mut array = PackedArray::<i32>::new();
    array.extend([10, 20, 30, 40, 50]);

    let sub = array.subarray(1, 4);
    assert_eq!(sub.as_slice(), &[20, 30, 40]);

    let sub_empty = array.subarray(2, 2);
    assert_eq!(sub_empty.len(), 0);

    // Test bounds clamping
    let sub_clamped = array.subarray(3, 100);
    assert_eq!(sub_clamped.as_slice(), &[40, 50]);
}

#[itest]
fn packed_array_generic_to_vec() {
    let original_vec = vec![GString::from("a"), GString::from("b"), GString::from("c")];
    let mut array = PackedArray::<GString>::new();
    array.extend(original_vec.iter().cloned());

    let converted_vec = array.to_vec();
    assert_eq!(converted_vec, original_vec);
}

#[itest]
fn packed_array_generic_clone_eq() {
    let mut array1 = PackedArray::<Vector3>::new();
    array1.extend([Vector3::new(1.0, 2.0, 3.0), Vector3::new(4.0, 5.0, 6.0)]);

    let array2 = array1.clone();
    assert_eq!(array1, array2);

    let mut array3 = PackedArray::<Vector3>::new();
    array3.extend([Vector3::new(1.0, 2.0, 3.0)]);
    assert_ne!(array1, array3);
}

#[itest]
fn packed_array_generic_count() {
    let mut array = PackedArray::<i32>::new();
    array.extend([1, 2, 1, 3, 1, 4]);

    assert_eq!(array.count(1), 3);
    assert_eq!(array.count(2), 1);
    assert_eq!(array.count(5), 0);
}

#[itest]
fn packed_array_generic_clear() {
    let mut array = PackedArray::<u8>::new();
    array.extend([1, 2, 3, 4, 5]);
    assert_eq!(array.len(), 5);

    array.clear();
    assert_eq!(array.len(), 0);
    assert!(array.is_empty());
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Generic helpers

fn test_generic_extend<T>(create_values: impl Fn(usize) -> Vec<T>) -> PackedArray<T>
where
    T: godot::meta::PackedArrayElement + Clone + PartialEq + std::fmt::Debug,
{
    let mut array = PackedArray::<T>::new();
    let values1 = create_values(3);
    let values2 = create_values(2);

    array.extend(values1.iter().cloned());
    assert_eq!(array.len(), 3);
    for (i, expected) in values1.iter().enumerate() {
        assert_eq!(&array[i], expected);
    }

    array.extend(values2);
    assert_eq!(array.len(), 5);

    array
}

fn test_generic_push<T>(create_values: impl Fn(usize) -> Vec<T>)
where
    T: godot::meta::PackedArrayElement + Clone + PartialEq + std::fmt::Debug,
{
    let mut array = PackedArray::<T>::new();
    let values = create_values(3);

    for value in values.iter() {
        array.push(owned_into_arg(value.clone()));
    }

    assert_eq!(array.len(), 3);
    for (i, expected) in values.iter().enumerate() {
        assert_eq!(&array[i], expected);
    }
}

fn test_generic_insert_remove<T>(create_values: impl Fn(usize) -> Vec<T>)
where
    T: godot::meta::PackedArrayElement + Clone + PartialEq + std::fmt::Debug,
{
    let mut array = PackedArray::<T>::new();
    let values = create_values(3);

    // Test insert
    array.push(owned_into_arg(values[0].clone()));
    array.push(owned_into_arg(values[2].clone()));
    array.insert(1, owned_into_arg(values[1].clone()));

    assert_eq!(array.len(), 3);
    for (i, expected) in values.iter().enumerate() {
        assert_eq!(&array[i], expected);
    }

    // Test remove
    let removed = array.remove(1);
    assert_eq!(removed, values[1]);
    assert_eq!(array.len(), 2);
    assert_eq!(array[0], values[0]);
    assert_eq!(array[1], values[2]);
}

fn test_generic_extend_array<T>(create_values: impl Fn(usize) -> Vec<T>)
where
    T: godot::meta::PackedArrayElement + Clone + PartialEq + std::fmt::Debug,
{
    let mut array1 = PackedArray::<T>::new();
    let values2 = create_values(3);
    let mut array2 = PackedArray::<T>::new();
    array2.extend(values2);
    let values3 = create_values(2);
    let mut array3 = PackedArray::<T>::new();
    array3.extend(values3.iter().cloned());

    array1.extend(create_values(2));
    array1.extend_array(&array2);
    array1.extend_array(&array3);

    assert_eq!(array1.len(), 7);
}

fn test_generic_sort_reverse<T>(mut create_values: impl FnMut() -> Vec<T>)
where
    T: godot::meta::PackedArrayElement + Clone + PartialEq + std::fmt::Debug,
{
    let values = create_values();
    if values.len() < 2 {
        return;
    }

    let mut array = PackedArray::<T>::new();
    array.extend(values.iter().cloned());
    let original_len = array.len();

    // Test reverse
    array.reverse();
    assert_eq!(array.len(), original_len);

    // Test sort (note: sort behavior depends on T's ordering)
    array.sort();
    assert_eq!(array.len(), original_len);
}

fn test_generic_resize_fill<T>(create_value: impl Fn() -> T)
where
    T: godot::meta::PackedArrayElement + Clone + PartialEq + std::fmt::Debug,
{
    let mut array = PackedArray::<T>::new();
    let test_value = create_value();

    // Test resize larger
    array.resize(5);
    assert_eq!(array.len(), 5);

    // Test fill
    array.fill(owned_into_arg(test_value.clone()));
    for i in 0..array.len() {
        assert_eq!(array[i], test_value);
    }

    // Test resize smaller
    array.resize(3);
    assert_eq!(array.len(), 3);
    for i in 0..array.len() {
        assert_eq!(array[i], test_value);
    }
}

fn test_generic_find_contains<T>(create_values: impl Fn() -> Vec<T>)
where
    T: godot::meta::PackedArrayElement + Clone + PartialEq + std::fmt::Debug,
{
    let values = create_values();
    if values.is_empty() {
        return;
    }

    let mut array = PackedArray::<T>::new();
    array.extend(values.iter().cloned());

    // Test contains
    assert!(array.contains(ref_to_arg(&values[0])));

    // Test find
    assert_eq!(array.find(ref_to_arg(&values[0]), None), Some(0));
    if values.len() > 1 {
        assert_eq!(array.find(ref_to_arg(&values[1]), None), Some(1));
    }
}
