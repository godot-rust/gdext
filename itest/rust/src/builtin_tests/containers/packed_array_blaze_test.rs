/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Tests for PackedArray::to_typed_array_blaze using RustVariant.

use godot::builtin::PackedFloat32Array;

use crate::framework::itest;

#[itest]
fn packed_f32_to_typed_array_blaze_correctness() {
    // Empty array.
    let packed = PackedFloat32Array::new();
    let array = packed.to_typed_array_blaze();
    assert_eq!(array.len(), 0);

    // Single element.
    let packed = PackedFloat32Array::from(&[3.15f32]);
    let array = packed.to_typed_array_blaze();
    assert_eq!(array.len(), 1);
    assert_eq!(array.at(0), 3.15f32);

    // Multiple elements.
    let packed = PackedFloat32Array::from(&[1.0, 2.5, -3.15, 0.0, 100.5]);
    let array = packed.to_typed_array_blaze();
    assert_eq!(array.len(), 5);
    assert_eq!(array.at(0), 1.0);
    assert_eq!(array.at(1), 2.5);
    assert_eq!(array.at(2), -3.15);
    assert_eq!(array.at(3), 0.0);
    assert_eq!(array.at(4), 100.5);

    // Many elements.
    let source: Vec<f32> = (0..1000).map(|i| i as f32 * 0.5).collect();
    let packed = PackedFloat32Array::from(&source[..]);
    let array = packed.to_typed_array_blaze();
    assert_eq!(array.len(), 1000);
    for i in 0..1000 {
        assert_eq!(array.at(i), i as f32 * 0.5);
    }
}

#[itest]
fn packed_f32_to_typed_array_blaze_vs_ffi() {
    // Verify that the blaze version produces the same result as the FFI version.
    let source: Vec<f32> = (0..100).map(|i| (i as f32).sin()).collect();
    let packed = PackedFloat32Array::from(&source[..]);

    let array_ffi = packed.to_typed_array();
    let array_blaze = packed.to_typed_array_blaze();

    assert_eq!(array_ffi.len(), array_blaze.len());
    for i in 0..array_ffi.len() {
        assert_eq!(array_ffi.at(i), array_blaze.at(i));
    }
}

#[itest]
fn packed_f32_to_typed_array_blaze_special_values() {
    // Test with special float values.
    let packed = PackedFloat32Array::from(&[
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        f32::MIN,
        f32::MAX,
        0.0,
        -0.0,
    ]);
    let array = packed.to_typed_array_blaze();

    assert_eq!(array.len(), 7);
    assert_eq!(array.at(0), f32::INFINITY);
    assert_eq!(array.at(1), f32::NEG_INFINITY);
    assert!(array.at(2).is_nan());
    assert_eq!(array.at(3), f32::MIN);
    assert_eq!(array.at(4), f32::MAX);
    assert_eq!(array.at(5), 0.0);
    assert_eq!(array.at(6), -0.0);
}
