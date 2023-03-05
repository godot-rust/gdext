/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::{expect_panic, itest};
use godot::prelude::*;

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
    assert_eq!(array.get(0), 1);
    assert_eq!(array.get(1), 2);
}

#[itest]
fn packed_array_from() {
    let array = PackedByteArray::from(&[1, 2]);

    assert_eq!(array.len(), 2);
    assert_eq!(array.get(0), 1);
    assert_eq!(array.get(1), 2);
}

#[itest]
fn packed_array_to_vec() {
    let array = PackedByteArray::from(&[1, 2]);
    assert_eq!(array.to_vec(), vec![1, 2]);
}

// #[itest]
// fn packed_array_into_iterator() {
//     let array = Array::from(&[1, 2]);
//     let mut iter = array.into_iter();
//     assert_eq!(iter.next(), Some(1));
//     assert_eq!(iter.next(), Some(2));
//     assert_eq!(iter.next(), None);
// }

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
    array.set(0, 3);
    assert_eq!(clone.get(0), 1);
}

#[itest]
fn packed_array_slice() {
    let array = PackedByteArray::from(&[1, 2, 3]);
    let slice = array.slice(1, 2);
    assert_eq!(slice.to_vec(), vec![2]);
}

#[itest]
fn packed_array_get() {
    let array = PackedByteArray::from(&[1, 2]);

    assert_eq!(array.get(0), 1);
    assert_eq!(array.get(1), 2);
    expect_panic("Array index 2 out of bounds: length is 2", || {
        array.get(2);
    });
}

#[itest]
fn packed_array_binary_search() {
    let array = PackedByteArray::from(&[1, 3]);

    assert_eq!(array.binary_search(0), 0);
    assert_eq!(array.binary_search(1), 0);
    assert_eq!(array.binary_search(2), 1);
    assert_eq!(array.binary_search(3), 1);
    assert_eq!(array.binary_search(4), 2);
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
fn packed_array_set() {
    let mut array = PackedByteArray::from(&[1, 2]);

    array.set(0, 3);
    assert_eq!(array.get(0), 3);

    expect_panic("Array index 2 out of bounds: length is 2", move || {
        array.set(2, 4);
    });
}

#[itest]
fn packed_array_push() {
    let mut array = PackedByteArray::from(&[1, 2]);

    array.push(3);

    assert_eq!(array.len(), 3);
    assert_eq!(array.get(2), 3);
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
fn packed_array_extend() {
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
fn packed_array_as_slice() {
    let vec = vec![1, 2];
    let array = PackedByteArray::from(vec.as_slice());
    assert_eq!(array.as_slice(), vec.as_slice());

    assert_eq!(
        PackedByteArray::new().as_slice(),
        &[],
        "Array is empty, but still a valid slice"
    );
}

#[itest]
fn packed_array_is_mut_unique() {
    let array1 = PackedByteArray::from(&[1, 2]);
    let mut array2 = array1.clone();
    let array3 = array1.to_variant();
    let array4 = godot::engine::Image::create_from_data(
        2,
        1,
        false,
        godot::engine::image::Format::FORMAT_R8,
        array1.clone(),
    )
    .unwrap();

    assert_eq!(
        array1.as_slice().as_ptr(),
        array2.as_slice().as_ptr(),
        "Arrays should share the same buffer"
    );
    assert_eq!(
        array1.as_slice().as_ptr(),
        array3.to::<PackedByteArray>().as_slice().as_ptr(),
        "Arrays should share the same buffer. Even when stored in a variant"
    );
    assert_eq!(
        array1.as_slice().as_ptr(),
        array4.get_data().as_slice().as_ptr(),
        "Arrays should share the same buffer. Even when stored in an Image"
    );

    // array2 should become an unique copy of array1
    // as mutable access triggers copy-on-write.
    array2.as_mut_slice();
    assert_ne!(
        array2.as_slice().as_ptr(),
        array1.as_slice().as_ptr(),
        "Arrays should not share the same buffer after a mutable access"
    );
    assert_ne!(
        array2.as_slice().as_ptr(),
        array3.to::<PackedByteArray>().as_slice().as_ptr(),
        "Arrays should not share the same buffer after a mutable access. Event when stored in a variant"
    );
    assert_ne!(
        array2.as_slice().as_ptr(),
        array4.get_data().as_slice().as_ptr(),
        "Arrays should not share the same buffer after a mutable access. Event when stored in an Image"
    );
    // These were not mutably accessed, so they should still share the same buffer.
    assert_eq!(
        array3.to::<PackedByteArray>().as_slice().as_ptr(),
        array4.get_data().as_slice().as_ptr(),
    );

    assert_eq!(
        array1.as_slice(),
        array2.as_slice(),
        "Array2 should be a copy of array1"
    );
}

#[derive(GodotClass, Debug)]
#[class(base=RefCounted)]
struct PackedArrayTest {
    array: PackedByteArray,
    _base: Base<RefCounted>,
}

#[godot_api]
impl PackedArrayTest {
    #[func]
    fn set_array(&mut self, array: PackedByteArray) {
        self.array = array;
    }

    #[func]
    fn get_array(&self) -> PackedByteArray {
        self.array.clone()
    }

    #[func]
    fn are_separate_buffer(&self, other: PackedByteArray) -> bool {
        self.array.as_slice().as_ptr() != other.as_slice().as_ptr()
    }

    #[func]
    fn do_mutable_access(&mut self) {
        self.array.as_mut_slice();
    }
}

#[godot_api]
impl GodotExt for PackedArrayTest {
    fn init(base: Base<Self::Base>) -> Self {
        Self {
            array: PackedByteArray::new(),
            _base: base,
        }
    }
}
