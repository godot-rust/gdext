/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::builtin::{
    varray, Color, GString, Plane, Quaternion, Rect2, Rect2i, Rid, RustMarshal, RustVariant,
    SetError, Variant, VariantType, Vector2, Vector2i, Vector3, Vector3i, Vector4, Vector4i,
};
use godot::meta::{GodotFfiVariant, ToGodot};

use crate::framework::itest;

// ------------------------------------------------------------------------------
// Generic Test Infrastructure
// ------------------------------------------------------------------------------

/// Trait for types testable with both FFI and RustMarshal.
trait VariantTestable: RustMarshal + GodotFfiVariant + PartialEq + std::fmt::Debug + Copy {
    /// Expected Variant type for this value.
    #[allow(dead_code)]
    fn expected_type() -> VariantType;

    /// Test values for this type (include edge cases).
    fn test_values() -> Vec<Self>;
}

/// Compare FFI vs RustMarshal for a value.
fn compare_ffi_vs_rust<T: VariantTestable>(value: T, test_name: &str) {
    // Create via FFI-only path.
    let ffi_variant = value.rust_to_variant_ffi();

    // Create via RustMarshal.
    let mut rust_variant = Variant::nil();
    RustVariant::view_mut(&mut rust_variant)
        .set_value(value)
        .expect("RustMarshal set should succeed");

    // Verify types match.
    assert_eq!(
        ffi_variant.get_type(),
        rust_variant.get_type(),
        "{test_name}: type mismatch"
    );

    // Test cross-compatibility: FFI reads RustMarshal variant.
    let ffi_from_rust: T = T::rust_from_variant_ffi(&rust_variant)
        .unwrap_or_else(|_| panic!("{test_name}: FFI cannot read RustMarshal variant"));
    assert_eq!(
        value, ffi_from_rust,
        "{test_name}: FFI read from RustMarshal variant produced different value"
    );

    // Test cross-compatibility: RustMarshal reads FFI variant.
    let mut ffi_copy = ffi_variant.clone();
    let rust_from_ffi = RustVariant::view_mut(&mut ffi_copy)
        .get_value::<T>()
        .unwrap_or_else(|| panic!("{test_name}: RustMarshal cannot read FFI variant"));
    assert_eq!(
        value, rust_from_ffi,
        "{test_name}: RustMarshal read from FFI variant produced different value"
    );
}

/// Macro to generate comprehensive tests for a type.
macro_rules! impl_variant_testable {
    ($T:ty, $variant_type:expr, [$($test_val:expr),+ $(,)?]) => {
        impl VariantTestable for $T {
            fn expected_type() -> VariantType {
                $variant_type
            }

            fn test_values() -> Vec<Self> {
                vec![$($test_val),+]
            }
        }

        paste::paste! {
            #[itest]
            fn [<rust_variant_ffi_compat_ $T:snake>]() {
                for (i, value) in <$T>::test_values().iter().enumerate() {
                    compare_ffi_vs_rust(*value, &format!("{}[{}]", stringify!($T), i));
                }
            }
        }
    };
}

// ------------------------------------------------------------------------------
// Implementations for RustMarshal Types
// ------------------------------------------------------------------------------

impl_variant_testable!(bool, VariantType::BOOL, [true, false]);

impl_variant_testable!(
    i64,
    VariantType::INT,
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

impl_variant_testable!(
    f64,
    VariantType::FLOAT,
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

impl_variant_testable!(
    Vector2i,
    VariantType::VECTOR2I,
    [
        Vector2i::ZERO,
        Vector2i::ONE,
        Vector2i::new(i32::MIN, i32::MAX),
        Vector2i::new(-2147483648, 2147483647),
        Vector2i::new(-1, -1),
        Vector2i::new(100, -200)
    ]
);

impl_variant_testable!(
    Vector3i,
    VariantType::VECTOR3I,
    [
        Vector3i::ZERO,
        Vector3i::ONE,
        Vector3i::new(-1, -2147483648, 2147483647),
        Vector3i::new(100, 200, 300),
        Vector3i::new(-100, -200, -300)
    ]
);

impl_variant_testable!(
    Vector4i,
    VariantType::VECTOR4I,
    [
        Vector4i::ZERO,
        Vector4i::ONE,
        Vector4i::new(-1, -2147483648, 2147483647, 1000),
        Vector4i::new(1, 2, 3, 4),
        Vector4i::new(-1, -2, -3, -4)
    ]
);

impl_variant_testable!(
    Color,
    VariantType::COLOR,
    [
        Color::from_rgba(0.0, 0.0, 0.0, 1.0),
        Color::from_rgba(1.0, 1.0, 1.0, 1.0),
        Color::from_rgba(0.7, 0.5, 0.3, 0.2),
        Color::from_rgba(0.0, 0.0, 0.0, 0.0)
    ]
);

impl_variant_testable!(
    Rect2i,
    VariantType::RECT2I,
    [
        Rect2i::default(),
        Rect2i::new(Vector2i::ZERO, Vector2i::new(100, 200)),
        Rect2i::new(Vector2i::new(-50, -50), Vector2i::new(100, 100))
    ]
);

impl_variant_testable!(Rid, VariantType::RID, [Rid::Invalid]);

// Precision-dependent types (fit in both single and double precision).
impl_variant_testable!(
    Vector2,
    VariantType::VECTOR2,
    [
        Vector2::ZERO,
        Vector2::ONE,
        Vector2::new(12.5, -3.5),
        Vector2::new(-100.0, 200.0)
    ]
);

impl_variant_testable!(
    Vector3,
    VariantType::VECTOR3,
    [
        Vector3::ZERO,
        Vector3::ONE,
        Vector3::new(1.5, 2.5, 3.5),
        Vector3::new(117.5, 100.0, -323.25),
        Vector3::new(-1.0, -2.0, -3.0)
    ]
);

impl_variant_testable!(
    Vector4,
    VariantType::VECTOR4,
    [
        Vector4::ZERO,
        Vector4::ONE,
        Vector4::new(-18.5, 24.75, -1.25, 777.875),
        Vector4::new(1.0, 2.0, 3.0, 4.0)
    ]
);

impl_variant_testable!(
    Quaternion,
    VariantType::QUATERNION,
    [
        Quaternion::default(),
        Quaternion::new(0.0, 0.0, 0.0, 1.0),
        Quaternion::new(0.5, 0.5, 0.5, 0.5)
    ]
);

impl_variant_testable!(
    Plane,
    VariantType::PLANE,
    [
        Plane::new(Vector3::new(1.0, 0.0, 0.0), 0.0),
        Plane::new(Vector3::new(0.0, 1.0, 0.0), 10.0),
        Plane::new(Vector3::new(0.0, 0.0, 1.0), -5.0)
    ]
);

impl_variant_testable!(
    Rect2,
    VariantType::RECT2,
    [
        Rect2::default(),
        Rect2::new(Vector2::ZERO, Vector2::new(100.0, 200.0)),
        Rect2::new(Vector2::new(-50.0, -50.0), Vector2::new(100.0, 100.0))
    ]
);

// ------------------------------------------------------------------------------
// Edge Case Tests
// ------------------------------------------------------------------------------

#[itest]
fn rust_variant_nil_type_mismatch() {
    let mut variant = Variant::nil();
    let view = RustVariant::view_mut(&mut variant);

    // Reading wrong type from nil should return None.
    assert_eq!(view.get_value::<i64>(), None);
    assert_eq!(view.get_value::<bool>(), None);
}

#[itest]
fn rust_variant_type_mismatch() {
    let mut variant = Variant::from(42i64);
    let view = RustVariant::view_mut(&mut variant);

    // Reading wrong type should return None.
    assert_eq!(view.get_value::<f64>(), None);
    assert_eq!(view.get_value::<bool>(), None);
}

#[itest]
fn rust_variant_ffi_special_floats() {
    // Test NaN, infinity separately due to PartialEq behavior.
    let test_values = [f64::NAN, f64::INFINITY, f64::NEG_INFINITY];

    for value in test_values {
        let ffi_variant = value.rust_to_variant_ffi();

        let mut rust_variant = Variant::nil();
        RustVariant::view_mut(&mut rust_variant)
            .set_value(value)
            .unwrap();

        // For NaN: Both should be NaN.
        // For infinity: Both should be equal.
        let ffi_extracted: f64 = f64::rust_from_variant_ffi(&ffi_variant).unwrap();
        let rust_extracted = RustVariant::view_mut(&mut rust_variant)
            .get_value::<f64>()
            .unwrap();

        if value.is_nan() {
            assert!(ffi_extracted.is_nan());
            assert!(rust_extracted.is_nan());
        } else {
            assert_eq!(ffi_extracted, rust_extracted);
        }
    }
}

// ------------------------------------------------------------------------------
// Existing Tests
// ------------------------------------------------------------------------------

#[itest]
fn rust_variant_getters() {
    // Nil - no generic getter for nil type.
    let mut nil_variant = Variant::nil();
    let view = RustVariant::view_mut(&mut nil_variant);
    assert_eq!(view.get_type(), VariantType::NIL);

    // Bool true.
    let mut bool_variant = Variant::from(true);
    let view = RustVariant::view_mut(&mut bool_variant);
    assert_eq!(view.get_type(), VariantType::BOOL);
    assert_eq!(view.get_value::<bool>(), Some(true));
    assert!(view.get_value::<i64>().is_none());

    // Bool false.
    let mut bool_variant = Variant::from(false);
    let view = RustVariant::view_mut(&mut bool_variant);
    assert_eq!(view.get_value::<bool>(), Some(false));

    // Int positive.
    let mut int_variant = Variant::from(42i64);
    let view = RustVariant::view_mut(&mut int_variant);
    assert_eq!(view.get_type(), VariantType::INT);
    assert_eq!(view.get_value::<i64>(), Some(42));
    assert!(view.get_value::<f64>().is_none());

    // Int edge case (min i64).
    let mut int_variant = Variant::from(i64::MIN);
    let view = RustVariant::view_mut(&mut int_variant);
    assert_eq!(view.get_value::<i64>(), Some(i64::MIN));

    // Int edge case (max i64).
    let mut int_variant = Variant::from(i64::MAX);
    let view = RustVariant::view_mut(&mut int_variant);
    assert_eq!(view.get_value::<i64>(), Some(i64::MAX));

    // Float.
    let mut float_variant = Variant::from(3.125f64);
    let view = RustVariant::view_mut(&mut float_variant);
    assert_eq!(view.get_type(), VariantType::FLOAT);
    assert_eq!(view.get_value::<f64>(), Some(3.125));
    assert!(view.get_value::<i64>().is_none());

    // Float negative.
    let mut float_variant = Variant::from(-1.5e10f64);
    let view = RustVariant::view_mut(&mut float_variant);
    assert_eq!(view.get_value::<f64>(), Some(-1.5e10));
}

#[itest]
fn rust_variant_setters() {
    // Start with nil, set to int, float, bool. Test multiple types in one test.
    let mut variant = Variant::nil();
    let view = RustVariant::view_mut(&mut variant);

    assert!(view.set_value(123i64).is_ok());
    assert_eq!(view.get_value::<i64>(), Some(123));
    assert_eq!(view.get_type(), VariantType::INT);

    // Change int to float.
    assert!(view.set_value(2.72f64).is_ok());
    assert_eq!(view.get_value::<f64>(), Some(2.72));
    assert_eq!(view.get_type(), VariantType::FLOAT);

    // Change float to bool.
    assert!(view.set_value(true).is_ok());
    assert_eq!(view.get_value::<bool>(), Some(true));
    assert_eq!(view.get_type(), VariantType::BOOL);

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
    let result = view.set_value(42i64);
    assert!(result.is_err());
    if let Err(SetError { current_type }) = result {
        assert_eq!(current_type, VariantType::STRING);
    }

    // Array is also a complex type.
    let mut array_variant = Variant::from(varray![1, 2, 3]);
    let view = RustVariant::view_mut(&mut array_variant);
    assert!(view.set_value(true).is_err());
}

#[itest]
fn rust_variant_roundtrip_with_ffi() {
    // Create variant via FFI, read with RustVariant. Test int and float.
    let mut variant = 42i64.to_variant();
    {
        let view = RustVariant::view_mut(&mut variant);
        assert_eq!(view.get_value::<i64>(), Some(42));
    }
    let extracted: i64 = variant.to();
    assert_eq!(extracted, 42);

    // Test float too.
    let mut variant = 99.5f64.to_variant();
    {
        let view = RustVariant::view_mut(&mut variant);
        assert_eq!(view.get_value::<f64>(), Some(99.5));
    }
    let extracted: f64 = variant.to();
    assert_eq!(extracted, 99.5);
}

#[itest]
fn rust_variant_set_then_ffi_extract() {
    // Set value via RustVariant, extract via FFI. Test int, bool, Vector2.
    let mut variant = Variant::nil();
    {
        let view = RustVariant::view_mut(&mut variant);
        view.set_value(-12345i64).unwrap();
    }
    let extracted: i64 = variant.to();
    assert_eq!(extracted, -12345);

    {
        let view = RustVariant::view_mut(&mut variant);
        view.set_value(false).unwrap();
    }
    let extracted: bool = variant.to();
    assert!(!extracted);

    // Test a precision-dependent type too.
    {
        let view = RustVariant::view_mut(&mut variant);
        view.set_value(Vector2::new(3.5, 7.25)).unwrap();
    }
    let extracted: Vector2 = variant.to();
    assert_eq!(extracted, Vector2::new(3.5, 7.25));
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

    let mut variant2 = variant1.clone();

    // Change variant1 to a different value.
    RustVariant::view_mut(&mut variant1)
        .set_value(modified)
        .unwrap();

    // variant2 should still have original value.
    assert_eq!(
        RustVariant::view_mut(&mut variant2).get_value::<Vector2i>(),
        Some(original)
    );
    assert_eq!(
        RustVariant::view_mut(&mut variant1).get_value::<Vector2i>(),
        Some(modified)
    );
}

#[itest]
fn rust_variant_clone_independence_ffi() {
    // Test that cloning creates independent copies via FFI (for comparison).
    let original = Vector2i::new(10, 20);
    let modified = Vector2i::new(99, 88);

    let mut variant1 = original.to_variant();
    let variant2 = variant1.clone();

    // Change variant1 to a different value.
    variant1 = modified.to_variant();

    // variant2 should still have original value.
    assert_eq!(variant2.to::<Vector2i>(), original);
    assert_eq!(variant1.to::<Vector2i>(), modified);
}
