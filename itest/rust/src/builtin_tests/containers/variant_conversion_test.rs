/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Tests for variant conversion paths (FFI vs RustMarshal optimization).

use godot::builtin::{Plane, Quaternion, Rect2, RustVariant, Vector2, Vector3, Vector4};
use godot::meta::GodotFfiVariant;

use crate::framework::itest;

/// Test that optimized and FFI-only paths produce identical results for precision-dependent types.
#[itest]
fn variant_conversion_paths_identical() {
    // Test Vector2.
    let v2 = Vector2::new(1.5, 2.5);
    let opt_variant = v2.rust_to_variant();
    let ffi_variant = v2.rust_to_variant_ffi();

    let opt_back: Vector2 = Vector2::rust_from_variant(&opt_variant).unwrap();
    let ffi_back: Vector2 = Vector2::rust_from_variant_ffi(&ffi_variant).unwrap();

    assert_eq!(v2, opt_back);
    assert_eq!(v2, ffi_back);
    assert_eq!(opt_back, ffi_back);

    // Test Vector3.
    let v3 = Vector3::new(1.0, 2.0, 3.0);
    let opt_variant = v3.rust_to_variant();
    let ffi_variant = v3.rust_to_variant_ffi();

    let opt_back: Vector3 = Vector3::rust_from_variant(&opt_variant).unwrap();
    let ffi_back: Vector3 = Vector3::rust_from_variant_ffi(&ffi_variant).unwrap();

    assert_eq!(v3, opt_back);
    assert_eq!(v3, ffi_back);
    assert_eq!(opt_back, ffi_back);

    // Test Vector4.
    let v4 = Vector4::new(1.0, 2.0, 3.0, 4.0);
    let opt_variant = v4.rust_to_variant();
    let ffi_variant = v4.rust_to_variant_ffi();

    let opt_back: Vector4 = Vector4::rust_from_variant(&opt_variant).unwrap();
    let ffi_back: Vector4 = Vector4::rust_from_variant_ffi(&ffi_variant).unwrap();

    assert_eq!(v4, opt_back);
    assert_eq!(v4, ffi_back);
    assert_eq!(opt_back, ffi_back);

    // Test Quaternion.
    let q = Quaternion::new(1.0, 0.0, 0.0, 0.0);
    let opt_variant = q.rust_to_variant();
    let ffi_variant = q.rust_to_variant_ffi();

    let opt_back: Quaternion = Quaternion::rust_from_variant(&opt_variant).unwrap();
    let ffi_back: Quaternion = Quaternion::rust_from_variant_ffi(&ffi_variant).unwrap();

    assert_eq!(q, opt_back);
    assert_eq!(q, ffi_back);
    assert_eq!(opt_back, ffi_back);

    // Test Plane.
    let p = Plane::new(Vector3::new(0.0, 1.0, 0.0), 5.0);
    let opt_variant = p.rust_to_variant();
    let ffi_variant = p.rust_to_variant_ffi();

    let opt_back: Plane = Plane::rust_from_variant(&opt_variant).unwrap();
    let ffi_back: Plane = Plane::rust_from_variant_ffi(&ffi_variant).unwrap();

    assert_eq!(p, opt_back);
    assert_eq!(p, ffi_back);
    assert_eq!(opt_back, ffi_back);

    // Test Rect2.
    let r = Rect2::new(Vector2::new(10.0, 20.0), Vector2::new(30.0, 40.0));
    let opt_variant = r.rust_to_variant();
    let ffi_variant = r.rust_to_variant_ffi();

    let opt_back: Rect2 = Rect2::rust_from_variant(&opt_variant).unwrap();
    let ffi_back: Rect2 = Rect2::rust_from_variant_ffi(&ffi_variant).unwrap();

    assert_eq!(r, opt_back);
    assert_eq!(r, ffi_back);
    assert_eq!(opt_back, ffi_back);
}

/// Test that precision-dependent types use RustMarshal optimization.
///
/// This verifies that variants created via `rust_to_variant()` can be read
/// by RustVariant directly, indicating the optimization is active.
#[itest]
fn precision_types_use_rust_marshal() {
    // Test Vector3.
    let test_vector = Vector3::new(1.0, 2.0, 3.0);
    let variant = test_vector.rust_to_variant();

    // If RustMarshal optimization is active, we should be able to
    // use RustVariant to read the value directly.
    let mut variant_mut = variant.clone();
    let view = RustVariant::view_mut(&mut variant_mut);

    let extracted = view.get_value::<Vector3>();
    assert_eq!(extracted, Some(test_vector));

    // Test Quaternion.
    let test_quat = Quaternion::new(1.0, 0.0, 0.0, 0.0);
    let variant = test_quat.rust_to_variant();

    let mut variant_mut = variant.clone();
    let view = RustVariant::view_mut(&mut variant_mut);

    let extracted = view.get_value::<Quaternion>();
    assert_eq!(extracted, Some(test_quat));
}

/// Test edge cases for precision-dependent types.
#[itest]
fn variant_conversion_edge_cases() {
    // Test with very small values.
    let small = Vector3::new(1e-10, -1e-10, 0.0);
    let variant = small.rust_to_variant();
    let back: Vector3 = Vector3::rust_from_variant(&variant).unwrap();
    assert_eq!(small, back);

    // Test with very large values.
    #[allow(clippy::approx_constant)]
    let large = Vector3::new(1e10, -1e10, 3.14159);
    let variant = large.rust_to_variant();
    let back: Vector3 = Vector3::rust_from_variant(&variant).unwrap();
    assert_eq!(large, back);

    // Test with zeros.
    let zeros = Vector4::new(0.0, 0.0, 0.0, 0.0);
    let variant = zeros.rust_to_variant();
    let back: Vector4 = Vector4::rust_from_variant(&variant).unwrap();
    assert_eq!(zeros, back);

    // Test with negative zeros (if applicable).
    let neg_zeros = Vector4::new(-0.0, -0.0, -0.0, -0.0);
    let variant = neg_zeros.rust_to_variant();
    let back: Vector4 = Vector4::rust_from_variant(&variant).unwrap();
    // Note: -0.0 == 0.0 in IEEE 754.
    assert_eq!(neg_zeros, back);
}
