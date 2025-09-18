/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
use std::fmt;

use godot::builtin::{
    vdict, Color, GString, PackedArray, PackedByteArray, PackedInt32Array, PackedStringArray,
    Variant, Vector2, Vector3, Vector4,
};
use godot::global::godot_str;
use godot::meta::{owned_into_arg, ref_to_arg, wrapped, PackedArrayElement, ToGodot};

use crate::framework::{expect_panic, itest};

/// Utility to run generic `PackedArray<T>` tests for multiple types `T`.
macro_rules! test {
    ($($t:ty),+ $(,)?) => {
        $(test::<$t>();)+
    };
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Constructor and basic creation tests

#[itest]
fn packed_array_default() {
    assert_eq!(PackedByteArray::default().len(), 0);
}

#[itest]
fn packed_array_new() {
    assert_eq!(PackedByteArray::new().len(), 0);
}

#[itest]
fn packed_array_clone() {
    fn test<T: Generator>() {
        let mut original = T::packed_n(3);
        let clone = original.clone();

        original[0] = T::gen(99);

        // CoW means original and clone are independent after modification.
        assert_eq!(clone.to_vec(), T::vec_n(3));
    }

    test!(u8, i32, GString, Color);
}

#[itest]
fn packed_array_from_iterator() {
    let array = PackedByteArray::from_iter([1, 2]);

    assert_eq!(array.len(), 2);
    assert_eq!(array[0], 1);
    assert_eq!(array[1], 2);
}

/* Enable once IntoIterator is implemented.
#[itest]
fn packed_array_into_iterator() {
    let array = PackedByteArray::from(&[1, 2]);
    let mut iter = array.into_iter();
    assert_eq!(iter.next(), Some(1));
    assert_eq!(iter.next(), Some(2));
    assert_eq!(iter.next(), None);
}
*/

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Element access and query tests

#[itest]
fn packed_array_get() {
    fn test<T: Generator>() {
        let array = T::packed_n(2);
        assert_eq!(array.get(0), Some(T::gen(0)));
        assert_eq!(array.get(1), Some(T::gen(1)));
        assert_eq!(array.get(2), None);

        let empty = T::packed_n(0);
        assert_eq!(empty.get(0), None);
    }

    test!(u8, i32, GString, Color);
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
fn packed_array_contains() {
    let array = PackedByteArray::from(&[1, 2, 3]);

    assert!(array.contains(1));
    assert!(array.contains(2));
    assert!(array.contains(3));
    assert!(!array.contains(0));
    assert!(!array.contains(4));
}

#[itest]
fn packed_array_find_contains() {
    fn test<T: Generator>() {
        let array = T::packed_n(4);
        let present = T::gen(2);
        let absent = T::gen(4); // Generator period >= 5.

        // contains().
        assert!(array.contains(ref_to_arg(&present)));
        assert!(!array.contains(ref_to_arg(&absent)));

        // find().
        assert_eq!(array.find(ref_to_arg(&present), None), Some(2));
        assert_eq!(array.find(ref_to_arg(&absent), None), None);
    }

    test!(u8, i32, GString, Color);
}

#[itest]
fn packed_array_count() {
    let array = PackedArray::from([1, 2, 1, 3, 1, 4]);

    assert_eq!(array.count(1), 3);
    assert_eq!(array.count(2), 1);
    assert_eq!(array.count(5), 0);
}

#[itest]
fn packed_array_len_is_empty() {
    let array = PackedByteArray::new();
    assert_eq!(array.len(), 0);
    assert!(array.is_empty());

    let array = PackedByteArray::from(&[1, 2, 3]);
    assert_eq!(array.len(), 3);
    assert!(!array.is_empty());
}

#[itest]
fn packed_array_eq() {
    fn test<T: Generator>() {
        let a = T::packed_n(3);
        let b = T::packed_n(3);
        let c = T::packed_n(2);

        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    test!(u8, i32, GString, Color);
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Modification tests

#[itest]
fn packed_array_clear() {
    let mut array = PackedArray::from([1, 2, 3, 4, 5]);
    assert_eq!(array.len(), 5);

    array.clear();
    assert_eq!(array.len(), 0);
    assert!(array.is_empty());
}

#[itest]
fn packed_array_push() {
    // Some concrete tests to test pass-by-value/pass-by-ref syntax.
    let mut ints = PackedByteArray::from(&[1, 2]);
    ints.push(3);
    assert_eq!(ints.len(), 3);
    assert_eq!(ints[2], 3);

    let mut strings = PackedStringArray::from(&[GString::from("a")]);
    strings.push("b");
    assert_eq!(strings.len(), 2);
    assert_eq!(strings[1], "b".into());

    fn test<T: Generator>() {
        let mut array = PackedArray::<T>::new();
        let mut vec = Vec::<T>::new();
        let values = T::vec_n(3);

        // push() multiple elements.
        for value in values.iter() {
            array.push(ref_to_arg(value));
            vec.push(value.clone());
        }

        assert_eq!(array.to_vec(), vec);
        for (i, expected) in values.iter().enumerate() {
            assert_eq!(&array[i], expected);
        }
    }

    test!(u8, i32, GString, Color);
}

#[itest]
fn packed_array_insert() {
    let mut array = PackedByteArray::from(&[1, 2]);

    array.insert(0, 3);
    assert_eq!(array.to_vec(), vec![3, 1, 2]);

    array.insert(3, 4);
    assert_eq!(array.to_vec(), vec![3, 1, 2, 4]);
}

#[itest]
fn packed_array_remove() {
    let mut array = PackedByteArray::from(&[1, 2, 3]);

    let removed = array.remove(1);
    assert_eq!(removed, 2);
    assert_eq!(array.to_vec(), vec![1, 3]);

    expect_panic("index out of bounds", || {
        array.remove(10);
    });
}

#[itest]
fn packed_array_insert_remove() {
    fn test<T: Generator>() {
        let mut array = PackedArray::<T>::new();
        let mut vec = Vec::<T>::new();
        let values = T::vec_n(3);

        // insert().
        array.push(ref_to_arg(&values[0]));
        vec.push(values[0].clone());

        array.push(ref_to_arg(&values[2]));
        vec.push(values[2].clone());

        array.insert(1, ref_to_arg(&values[1]));
        vec.insert(1, values[1].clone());

        assert_eq!(array.to_vec(), vec);

        for (i, expected) in values.iter().enumerate() {
            assert_eq!(&array[i], expected);
        }

        // remove().
        let removed_array = array.remove(1);
        let removed_vec = vec.remove(1);

        assert_eq!(removed_array, removed_vec);
        assert_eq!(removed_array, values[1]);
        assert_eq!(array.to_vec(), vec);
        assert_eq!(array[0], values[0]);
        assert_eq!(array[1], values[2]);
    }

    test!(u8, i32, GString, Color);
}

#[itest]
fn packed_array_fill() {
    let mut array = PackedByteArray::from(&[1, 2, 3]);

    array.fill(42);
    assert_eq!(array.to_vec(), vec![42, 42, 42]);

    let mut empty = PackedByteArray::new();
    empty.fill(42);
    assert_eq!(empty.len(), 0);
}

#[itest]
fn packed_array_resize() {
    let mut array = PackedByteArray::from(&[1, 2, 3]);

    // Resize larger
    array.resize(5);
    assert_eq!(array.len(), 5);
    assert_eq!(array[0], 1);
    assert_eq!(array[1], 2);
    assert_eq!(array[2], 3);
    // New elements are default-constructed (0 for u8)
    assert_eq!(array[3], 0);
    assert_eq!(array[4], 0);

    // Resize smaller
    array.resize(2);
    assert_eq!(array.len(), 2);
    assert_eq!(array.to_vec(), vec![1, 2]);
}

#[itest]
fn packed_array_resize_fill() {
    // TODO: PackedArray::resize() should be split into growing/shrinking API, see Array.

    fn test<T: Generator>() {
        let mut array = PackedArray::<T>::new();
        let mut vec = Vec::<T>::new();
        let elem = T::gen(0);

        // resize() growing.
        // PackedArray::resize() fills with default-constructed values. Compensate for it in vec.
        array.resize(5);
        vec.resize(5, T::default());

        // Test equality after growing resize.
        assert_eq!(array.to_vec(), vec);

        // fill().
        array.fill(ref_to_arg(&elem));
        vec.fill(elem.clone());

        assert_eq!(array.to_vec(), vec);
        for i in 0..array.len() {
            assert_eq!(array[i], elem);
        }

        // resize() shrinking.
        array.resize(3);
        vec.resize(3, elem.clone()); // Param is ignored when shrinking.

        assert_eq!(array.to_vec(), vec);
        for i in 0..array.len() {
            assert_eq!(array[i], elem);
        }
    }

    test!(u8, i32, GString, Color);
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
fn packed_array_extend_generic() {
    fn test<T: Generator>() {
        // extend().
        let mut array = T::packed_n(3);
        array.extend([T::gen(3), T::gen(4)]);
        assert_eq!(array.to_vec(), T::vec_n(5));
    }

    test!(u8, i32, GString, Color);
}

#[itest]
fn packed_array_extend_array() {
    fn test<T: Generator>() {
        // extend_array() with self.
        let mut array = T::packed_n(2);
        array.extend_array(&array.clone()); // clone() to avoid aliasing issues.
        assert_eq!(array.len(), 4);
        assert_eq!(array[0], T::gen(0));
        assert_eq!(array[1], T::gen(1));
        assert_eq!(array[2], T::gen(0));
        assert_eq!(array[3], T::gen(1));

        // extend_array() with another array.
        let mut array = T::packed_n(2);
        array.extend_array(&T::packed_n(3));
        assert_eq!(array.len(), 5);
        assert_eq!(array[0], T::gen(0));
        assert_eq!(array[1], T::gen(1));
        assert_eq!(array[2], T::gen(0));
        assert_eq!(array[3], T::gen(1));
        assert_eq!(array[4], T::gen(2));
    }

    test!(u8, i32, GString, Color);
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Slice and view tests

#[itest]
fn packed_array_subarray() {
    let array = PackedArray::from([10, 20, 30, 40, 50]);

    let sub = array.subarray(1..4);
    assert_eq!(sub.as_slice(), &[20, 30, 40]);

    let endless = array.subarray(2..);
    assert_eq!(endless.as_slice(), &[30, 40, 50]);

    let negative_sub = array.subarray(wrapped(-4..-2));
    assert_eq!(negative_sub.as_slice(), &[20, 30]);

    let sub_empty = array.subarray(2..2);
    assert_eq!(sub_empty.len(), 0);

    // Half-open range: end index is clamped to len.
    let sub_clamped = array.subarray(3..100);
    assert_eq!(sub_clamped.as_slice(), &[40, 50]);
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

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Search tests

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
fn packed_array_find_rfind_comprehensive() {
    fn test<T: Generator>() {
        let mut array = PackedArray::<T>::new();
        let mut vec = Vec::<T>::new();

        // Pattern: [gen(0), gen(1), gen(0), gen(2), gen(1), gen(0)], with deliberate duplicates.
        let pattern = [0, 1, 0, 2, 1, 0];
        for &idx in &pattern {
            let val = T::gen(idx);
            array.push(ref_to_arg(&val));
            vec.push(val);
        }

        assert_eq!(array.to_vec(), vec);

        let val0 = T::gen(0);
        let val1 = T::gen(1);
        let val_nonexistent = T::gen(3); // Non-existent value.

        assert_eq!(array.find(ref_to_arg(&val0), None), Some(0)); // find(None) -> first occurrence.
        assert_eq!(array.rfind(ref_to_arg(&val0), None), Some(5)); // rfind(None) -> last occurrence.
        assert_eq!(array.find(ref_to_arg(&val0), Some(1)), Some(2)); // find(start).
        assert_eq!(array.rfind(ref_to_arg(&val1), Some(3)), Some(1)); // rfind(start).
        assert_eq!(array.find(ref_to_arg(&val_nonexistent), None), None); // find(non_existent) -> None.
    }

    test!(u8, i32, GString, Color);
}

#[itest]
fn packed_array_bsearch() {
    // Concret tests for i32.
    let array = PackedInt32Array::from(&[1, 3]);
    assert_eq!(array.bsearch(0), 0);
    assert_eq!(array.bsearch(1), 0);
    assert_eq!(array.bsearch(2), 1);
    assert_eq!(array.bsearch(3), 1);
    assert_eq!(array.bsearch(4), 2);

    fn test<T: Generator + Ord>() {
        let mut values = T::vec_n(10);
        values.sort(); // Ensure sorted for binary search.

        let mut array = PackedArray::<T>::new();
        let mut vec = Vec::<T>::new();

        array.extend(values.iter().cloned());
        vec.extend(values.iter().cloned());

        assert_eq!(array.to_vec(), vec);

        // bsearch(existing value) -> correct index.
        let test_value = &values[3];
        let array_result = array.bsearch(ref_to_arg(test_value));
        let vec_result = vec.binary_search(test_value).unwrap();
        assert_eq!(array_result, vec_result);
        assert_eq!(array_result, 3);

        // bsearch(absent value) -> correct insertion point.
        let non_existent = T::gen(values.len() + 10);
        let array_insertion = array.bsearch(ref_to_arg(&non_existent));
        let vec_insertion = vec
            .binary_search(&non_existent)
            .unwrap_or_else(|insertion_point| insertion_point);
        assert_eq!(array_insertion, vec_insertion);
    }

    test!(u8, i32);
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Reordering tests

#[itest]
fn packed_array_sort_reverse() {
    fn test<T: Generator + Ord>() {
        let mut array = T::packed_n(5);
        let mut vec = T::vec_n(5);

        // reverse(): same op on both.
        array.reverse();
        vec.reverse();
        assert_eq!(array.to_vec(), vec);

        // sort(): same op on both.
        array.sort();
        vec.sort();
        assert_eq!(array.to_vec(), vec);
    }

    test!(u8, i32, GString); // only Ord types.
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Display tests

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
// Conversion and trait tests

#[itest]
fn packed_array_to_vec() {
    fn test<T: Generator>() {
        let array = T::packed_n(3);
        let vec = array.to_vec();

        assert_eq!(vec, T::vec_n(3));
    }

    test!(u8, i32, GString, Color);
}

#[itest]
fn packed_array_from_vec() {
    fn test<T: Generator>() {
        let vec = T::vec_n(3);
        let array = PackedArray::<T>::from(vec.clone());

        assert_eq!(array.len(), vec.len());
        assert_eq!(array.to_vec(), vec);
    }

    test!(u8, i32, GString, Color);
}

#[itest]
fn packed_array_from_array() {
    fn test<T: Generator>() {
        let values = [T::gen(0), T::gen(1), T::gen(2)];
        let array = PackedArray::<T>::from(values.clone());

        assert_eq!(array.len(), values.len());
        for (i, expected) in values.iter().enumerate() {
            assert_eq!(array[i], *expected);
        }
    }

    test!(u8, i32, GString, Color);
}

#[itest]
fn packed_array_all_types() {
    fn test<T: PackedArrayElement + Default + PartialEq + fmt::Debug>() {
        let val = T::default();

        let mut array: PackedArray<T> = PackedArray::new();
        array.push(ref_to_arg(&val));
        array.push(owned_into_arg(val));
        array.extend([T::default()]);

        assert_eq!(array.len(), 3);
        assert_eq!(array[0], T::default());
        assert_eq!(array[1], T::default());
        assert_eq!(array[2], T::default());
    }

    test!(u8, i32, i64, f32, f64, Vector2, Vector3, Color, GString);

    #[cfg(since_api = "4.3")]
    test!(Vector4);
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Generator trait and implementations

/// Deterministic value generator for tests.
trait Generator: PackedArrayElement + Default + PartialEq + fmt::Debug {
    /// Deterministically generate a value depending on the index.
    ///
    /// Values may repeat, but the period must be at least 5.
    fn gen(index: usize) -> Self;

    /// Generate a `Vec` of `gen(0)`, `gen(1)`, ..., `gen(n-1)`.
    fn vec_n(n: usize) -> Vec<Self>
    where
        Self: Sized,
    {
        (0..n).map(|i| Self::gen(i)).collect()
    }

    /// Generate a `PackedArray` of `gen(0)`, `gen(1)`, ..., `gen(n-1)`.
    fn packed_n(n: usize) -> PackedArray<Self>
    where
        Self: Sized,
    {
        let iter = (0..n).map(|i| Self::gen(i));
        PackedArray::from_iter(iter)
    }
}

impl Generator for u8 {
    fn gen(index: usize) -> Self {
        index as u8
    }
}

impl Generator for i32 {
    fn gen(index: usize) -> Self {
        index as i32
    }
}

impl Generator for GString {
    fn gen(index: usize) -> Self {
        godot_str!("test_{}", index)
    }
}

impl Generator for Color {
    fn gen(index: usize) -> Self {
        let colors = [
            Color::RED,
            Color::GREEN,
            Color::BLUE,
            Color::YELLOW,
            Color::CYAN,
            Color::MAGENTA,
        ];
        colors[index % colors.len()]
    }
}
