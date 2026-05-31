/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::builtin::__test_only::{RustMarshal, RustVariant, SetError};
use godot::builtin::{
    Array, Color, GString, Plane, Quaternion, Rect2, Rect2i, Rid, Variant, VariantType, Vector2,
    Vector2i, Vector3, Vector3i, Vector4, Vector4i, varray,
};
use godot::meta::{FromGodot, ToGodot};

use crate::framework::itest;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Generic Test Infrastructure

/// Verify variant round-trip: public conversion API, direct `RustVariant` view, and `set_value` path all agree.
fn verify_variant_roundtrip<T>(value: T, index: usize)
where
    T: RustMarshal + FromGodot + ToGodot + PartialEq + std::fmt::Debug + Copy,
{
    let label = || format!("{}[{}]", std::any::type_name::<T>(), index);
    let variant = value.to_variant();

    assert_eq!(
        variant.get_type(),
        <T as RustMarshal>::VARIANT_TYPE,
        "{}: unexpected variant type",
        label(),
    );

    // Public path: full conversion API.
    let via_public = T::from_variant(&variant);
    assert_eq!(
        value,
        via_public,
        "{}: public from_variant mismatch",
        label()
    );

    // Direct path: inline memory access via RustVariant (read-only view).
    let variant_copy = variant.clone();
    let via_view = RustVariant::view(&variant_copy).get_value::<T>();
    assert_eq!(
        Some(value),
        via_view,
        "{}: RustVariant view mismatch",
        label()
    );

    // Set path: write via RustVariant, then read both ways.
    let mut set_variant = Variant::nil();
    RustVariant::view_mut(&mut set_variant)
        .set_value(value)
        .unwrap_or_else(|_| panic!("{}: set_value failed on nil", label()));
    assert_eq!(
        Some(value),
        RustVariant::view(&set_variant).get_value::<T>(),
        "{}: set_value -> get_value mismatch",
        label(),
    );
    assert_eq!(
        value,
        T::from_variant(&set_variant),
        "{}: set_value -> FromGodot mismatch",
        label(),
    );
}

/// Macro to generate comprehensive tests for a type.
macro_rules! impl_variant_test {
    ($T:ty, $test_name:ident, [$($test_val:expr),+ $(,)?]) => {
        #[itest]
        fn $test_name() {
            for (i, value) in [$($test_val),+].iter().enumerate() {
                verify_variant_roundtrip::<$T>(*value, i);
            }
        }
    };
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Implementations for RustMarshal Types

impl_variant_test!(bool, rust_variant_roundtrip_bool, [true, false]);

impl_variant_test!(
    i64,
    rust_variant_roundtrip_i64,
    [
        0,
        1,
        -1,
        42,
        -12345,
        i64::MIN,
        i64::MAX,
        i64::MIN + 1,
        i64::MAX - 1
    ]
);

impl_variant_test!(
    f64,
    rust_variant_roundtrip_f64,
    [
        0.0,
        1.0,
        -1.0,
        3.125,
        -1.5e10,
        99.5,
        f64::MIN,
        f64::MAX,
        f64::EPSILON,
        -f64::EPSILON
    ]
);

impl_variant_test!(
    Vector2i,
    rust_variant_roundtrip_vector2i,
    [
        Vector2i::ZERO,
        Vector2i::ONE,
        Vector2i::new(i32::MIN, i32::MAX),
        Vector2i::new(-1, -1),
        Vector2i::new(100, -200)
    ]
);

impl_variant_test!(
    Vector3i,
    rust_variant_roundtrip_vector3i,
    [
        Vector3i::ZERO,
        Vector3i::ONE,
        Vector3i::new(-1, i32::MIN, i32::MAX),
        Vector3i::new(100, 200, 300),
        Vector3i::new(-100, -200, -300)
    ]
);

impl_variant_test!(
    Vector4i,
    rust_variant_roundtrip_vector4i,
    [
        Vector4i::ZERO,
        Vector4i::ONE,
        Vector4i::new(-1, i32::MIN, i32::MAX, 1000),
        Vector4i::new(1, 2, 3, 4),
        Vector4i::new(-1, -2, -3, -4)
    ]
);

impl_variant_test!(
    Color,
    rust_variant_roundtrip_color,
    [
        Color::from_rgba(0.0, 0.0, 0.0, 1.0),
        Color::from_rgba(1.0, 1.0, 1.0, 1.0),
        Color::from_rgba(0.7, 0.5, 0.3, 0.2),
        Color::from_rgba(0.0, 0.0, 0.0, 0.0)
    ]
);

impl_variant_test!(
    Rect2i,
    rust_variant_roundtrip_rect2i,
    [
        Rect2i::default(),
        Rect2i::new(Vector2i::ZERO, Vector2i::new(100, 200)),
        Rect2i::new(Vector2i::new(-50, -50), Vector2i::new(100, 100))
    ]
);

impl_variant_test!(
    Rid,
    rust_variant_roundtrip_rid,
    [
        Rid::Invalid,
        Rid::new(1),
        Rid::new(12345),
        Rid::new(u64::MAX),
    ]
);

// Precision-dependent types (fit in both single and double precision).
impl_variant_test!(
    Vector2,
    rust_variant_roundtrip_vector2,
    [
        Vector2::ZERO,
        Vector2::ONE,
        Vector2::new(12.5, -3.5),
        Vector2::new(-100.0, 200.0)
    ]
);

impl_variant_test!(
    Vector3,
    rust_variant_roundtrip_vector3,
    [
        Vector3::ZERO,
        Vector3::ONE,
        Vector3::new(1.5, 2.5, 3.5),
        Vector3::new(117.5, 100.0, -323.25),
        Vector3::new(-1.0, -2.0, -3.0)
    ]
);

impl_variant_test!(
    Vector4,
    rust_variant_roundtrip_vector4,
    [
        Vector4::ZERO,
        Vector4::ONE,
        Vector4::new(-18.5, 24.75, -1.25, 777.875),
        Vector4::new(1.0, 2.0, 3.0, 4.0)
    ]
);

impl_variant_test!(
    Quaternion,
    rust_variant_roundtrip_quaternion,
    [
        Quaternion::default(),
        Quaternion::new(0.0, 0.0, 0.0, 1.0),
        Quaternion::new(0.5, 0.5, 0.5, 0.5)
    ]
);

impl_variant_test!(
    Plane,
    rust_variant_roundtrip_plane,
    [
        Plane::new(Vector3::new(1.0, 0.0, 0.0), 0.0),
        Plane::new(Vector3::new(0.0, 1.0, 0.0), 10.0),
        Plane::new(Vector3::new(0.0, 0.0, 1.0), -5.0)
    ]
);

impl_variant_test!(
    Rect2,
    rust_variant_roundtrip_rect2,
    [
        Rect2::default(),
        Rect2::new(Vector2::ZERO, Vector2::new(100.0, 200.0)),
        Rect2::new(Vector2::new(-50.0, -50.0), Vector2::new(100.0, 100.0))
    ]
);

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Edge Case Tests

#[itest]
fn rust_variant_type_mismatch() {
    // Reading wrong type returns None, both for nil source and typed source.
    let nil_variant = Variant::nil();
    let view = RustVariant::view(&nil_variant);
    assert_eq!(view.get_type_unchecked(), VariantType::NIL);
    assert_eq!(view.get_value::<i64>(), None);
    assert_eq!(view.get_value::<bool>(), None);

    let int_variant = Variant::from(42i64);
    let view = RustVariant::view(&int_variant);
    assert_eq!(view.get_value::<f64>(), None);
    assert_eq!(view.get_value::<bool>(), None);

    let bool_variant = Variant::from(true);
    let view = RustVariant::view(&bool_variant);
    assert_eq!(view.get_value::<i64>(), None);
    assert_eq!(view.get_value::<f64>(), None);
}

#[itest]
fn rust_variant_special_floats() {
    // NaN/infinity round-trip through both the public API and the direct RustVariant view.
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let variant = value.to_variant();
        let via_view = RustVariant::view(&variant).get_value::<f64>().unwrap();
        let via_public = f64::from_variant(&variant);

        if value.is_nan() {
            assert!(via_view.is_nan());
            assert!(via_public.is_nan());
        } else {
            assert_eq!(via_view, value);
            assert_eq!(via_public, value);
        }
    }
}

#[itest]
fn rust_variant_setters() {
    // Start with nil, set to int, float, bool. Test multiple types in one test.
    let mut variant = Variant::nil();
    let view = RustVariant::view_mut(&mut variant);

    assert!(view.set_value(123i64).is_ok());
    assert_eq!(view.get_value::<i64>(), Some(123));
    assert_eq!(view.get_type_unchecked(), VariantType::INT);

    // Change int to float.
    assert!(view.set_value(2.72f64).is_ok());
    assert_eq!(view.get_value::<f64>(), Some(2.72));
    assert_eq!(view.get_type_unchecked(), VariantType::FLOAT);

    // Change float to bool.
    assert!(view.set_value(true).is_ok());
    assert_eq!(view.get_value::<bool>(), Some(true));
    assert_eq!(view.get_type_unchecked(), VariantType::BOOL);

    // Verify the underlying Variant was actually modified via FFI.
    let extracted: bool = variant.to();
    assert!(extracted);
}

#[itest]
fn rust_variant_setters_reject_complex_types() {
    // String is a complex type that needs destruction.
    let mut string_variant = Variant::from(GString::from("hello"));
    let view = RustVariant::view_mut(&mut string_variant);

    // Should fail because String needs destruction.
    let SetError { current_type } = view.set_value(42i64).unwrap_err();
    assert_eq!(current_type, VariantType::STRING);

    // Array is also a complex type.
    let mut array_variant = Variant::from(varray![1, 2, 3]);
    let view = RustVariant::view_mut(&mut array_variant);
    assert!(view.set_value(true).is_err());
}

#[itest]
fn rust_variant_clone_independence() {
    // Test that cloning creates independent copies via RustMarshal.
    let original = Vector2i::new(10, 20);
    let modified = Vector2i::new(99, 88);

    let mut variant1 = Variant::nil();
    RustVariant::view_mut(&mut variant1)
        .set_value(original)
        .unwrap();

    let variant2 = variant1.clone();

    // Change variant1 to a different value.
    RustVariant::view_mut(&mut variant1)
        .set_value(modified)
        .unwrap();

    // variant2 should still have original value.
    assert_eq!(
        RustVariant::view(&variant2).get_value::<Vector2i>(),
        Some(original)
    );
    assert_eq!(
        RustVariant::view(&variant1).get_value::<Vector2i>(),
        Some(modified)
    );
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Cross-FFI Layout Tests

/// Verify that Rust-created variants (RustMarshal path) are byte-compatible with Godot-created variants (FFI path).
///
/// `varray![x]` builds the Variant via Godot's C++ side; reading back via `RustVariant::view` confirms layout agreement
/// across both directions.
#[itest]
fn rust_variant_layout_matches_godot_i64() {
    // Godot writes the Variant; Rust reads via RustVariant.
    let godot_created: Variant = varray![42_i64].at(0);
    let rust_created = 42_i64.to_variant();

    let godot_view = RustVariant::view(&godot_created);
    let rust_view = RustVariant::view(&rust_created);

    assert_eq!(
        godot_view.get_type_unchecked(),
        rust_view.get_type_unchecked()
    );
    assert_eq!(godot_view.get_value::<i64>(), Some(42_i64));
    assert_eq!(rust_view.get_value::<i64>(), Some(42_i64));
}

#[itest]
fn rust_variant_layout_matches_godot_vector3() {
    let v = Vector3::new(1.5, 2.5, 3.5);

    let godot_created: Variant = varray![v].at(0);
    let rust_created = v.to_variant();

    assert_eq!(
        RustVariant::view(&godot_created).get_value::<Vector3>(),
        Some(v),
    );
    assert_eq!(
        RustVariant::view(&rust_created).get_value::<Vector3>(),
        Some(v),
    );
}

/// Verify that Rust-written variants are readable by Godot (reverse direction).
///
/// Stores a Rust-marshalled Variant into a Godot `Array`, then retrieves it via the public API to confirm Godot
/// can decode what Rust wrote.
#[itest]
fn rust_variant_readable_by_godot() {
    let rust_variant = 12345_i64.to_variant();

    let mut arr: Array<Variant> = Array::new();
    arr.push(&rust_variant);

    // Godot reads back from its own storage.
    let retrieved = arr.at(0);
    assert_eq!(i64::from_variant(&retrieved), 12345_i64);
}
