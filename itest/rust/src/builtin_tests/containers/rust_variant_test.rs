/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::builtin::__test_only::{RustMarshal, RustVariant};
use godot::builtin::{
    Array, Callable, Color, Plane, Quaternion, Rect2, Rect2i, Rid, Variant, VariantType, Vector2,
    Vector2i, Vector3, Vector3i, Vector4, Vector4i, varray,
};
use godot::meta::{FromGodot, ToGodot};

use crate::framework::itest;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Generic Test Infrastructure

/// Verify variant round-trip: public conversion API and direct `RustVariant` view agree.
fn verify_variant_roundtrip<T>(value: T, index: usize)
where
    T: RustMarshal + FromGodot + ToGodot + PartialEq + std::fmt::Debug + Copy,
{
    let label = format!("{}[{}]", std::any::type_name::<T>(), index);
    let variant = value.to_variant();

    assert_eq!(
        variant.get_type(),
        <T as RustMarshal>::VARIANT_TYPE,
        "{label}: unexpected variant type"
    );

    // Public path: full conversion API.
    let via_public = T::from_variant(&variant);
    assert_eq!(via_public, value, "{label}: public from_variant mismatch");

    // Direct path: inline memory access via read-only `RustVariant` view.
    let via_view = RustVariant::view(&variant).get_value::<T>();
    assert_eq!(via_view, Some(value), "{label}: RustVariant view mismatch");
}

/// Rust encode + decode (roundtrip test) isn't sufficient to ensure that we match *Godot's* layout, so additionally check `stringify()` via FFI.
fn verify_godot_side_read<T: ToGodot>(value: T, expected: &str) {
    let variant = value.to_variant();
    assert_eq!(variant.stringify().to_string(), expected);
}

/// Macro to generate comprehensive tests for a type.
macro_rules! declare_variant_test {
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

declare_variant_test!(bool, rust_variant_roundtrip_bool, [true, false]);

declare_variant_test!(
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

declare_variant_test!(
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

declare_variant_test!(
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

declare_variant_test!(
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

declare_variant_test!(
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

declare_variant_test!(
    Color,
    rust_variant_roundtrip_color,
    [
        Color::from_rgba(0.0, 0.0, 0.0, 1.0),
        Color::from_rgba(1.0, 1.0, 1.0, 1.0),
        Color::from_rgba(0.7, 0.5, 0.3, 0.2),
        Color::from_rgba(0.0, 0.0, 0.0, 0.0)
    ]
);

declare_variant_test!(
    Rect2i,
    rust_variant_roundtrip_rect2i,
    [
        Rect2i::default(),
        Rect2i::new(Vector2i::ZERO, Vector2i::new(100, 200)),
        Rect2i::new(Vector2i::new(-50, -50), Vector2i::new(100, 100))
    ]
);

declare_variant_test!(
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
declare_variant_test!(
    Vector2,
    rust_variant_roundtrip_vector2,
    [
        Vector2::ZERO,
        Vector2::ONE,
        Vector2::new(12.5, -3.5),
        Vector2::new(-100.0, 200.0)
    ]
);

declare_variant_test!(
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

declare_variant_test!(
    Vector4,
    rust_variant_roundtrip_vector4,
    [
        Vector4::ZERO,
        Vector4::ONE,
        Vector4::new(-18.5, 24.75, -1.25, 777.875),
        Vector4::new(1.0, 2.0, 3.0, 4.0)
    ]
);

declare_variant_test!(
    Quaternion,
    rust_variant_roundtrip_quaternion,
    [
        Quaternion::default(),
        Quaternion::new(0.0, 0.0, 0.0, 1.0),
        Quaternion::new(0.5, 0.5, 0.5, 0.5)
    ]
);

declare_variant_test!(
    Plane,
    rust_variant_roundtrip_plane,
    [
        Plane::new(Vector3::new(1.0, 0.0, 0.0), 0.0),
        Plane::new(Vector3::new(0.0, 1.0, 0.0), 10.0),
        Plane::new(Vector3::new(0.0, 0.0, 1.0), -5.0)
    ]
);

declare_variant_test!(
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
    // Reading wrong type returns None, for nil and populated variants.
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
    // NaN/infinity round-trip through both public API and direct RustVariant view.
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

// Complex types keep FFI path: `RustVariant::view` still reports their type, but they are not `RustMarshal` and thus never bit-copied.
#[itest]
fn rust_variant_complex_types_not_inplace() {
    let string_variant = Variant::from("hello");
    assert_eq!(
        RustVariant::view(&string_variant).get_type_unchecked(),
        VariantType::STRING
    );

    let array_variant = Variant::from(varray![1, 2, 3]);
    assert_eq!(
        RustVariant::view(&array_variant).get_type_unchecked(),
        VariantType::ARRAY
    );
}

// `Variant::clone()` bit-copies in-place variants instead of calling `variant_new_copy`; the copy must be independent of the source.
#[itest]
fn rust_variant_clone_independence() {
    let original = Vector2i::new(10, 20);
    let modified = Vector2i::new(99, 88);

    let mut variant1 = original.to_variant();
    let variant2 = variant1.clone();
    variant1 = modified.to_variant();

    assert_eq!(
        RustVariant::view(&variant2).get_value::<Vector2i>(),
        Some(original)
    );
    assert_eq!(
        RustVariant::view(&variant1).get_value::<Vector2i>(),
        Some(modified)
    );
}

// Godot-side decode of every RustMarshal type: catches a wrong data offset or field transposition that a Rust-only round-trip would miss.
#[itest]
fn rust_variant_godot_side_read_all_types() {
    verify_godot_side_read(true, "true");
    verify_godot_side_read(false, "false");
    verify_godot_side_read(i64::MIN, "-9223372036854775808");
    verify_godot_side_read(i64::MAX, "9223372036854775807");
    verify_godot_side_read(2.5_f64, "2.5");
    verify_godot_side_read(-0.125_f64, "-0.125");

    verify_godot_side_read(Vector2i::new(1, -2), "(1, -2)");
    verify_godot_side_read(Vector3i::new(1, -2, 3), "(1, -2, 3)");
    verify_godot_side_read(Vector4i::new(1, -2, 3, -4), "(1, -2, 3, -4)");
    verify_godot_side_read(Vector2::new(1.5, -2.5), "(1.5, -2.5)");
    verify_godot_side_read(Vector3::new(1.5, -2.5, 3.5), "(1.5, -2.5, 3.5)");
    verify_godot_side_read(Vector4::new(1.5, -2.5, 3.5, -4.5), "(1.5, -2.5, 3.5, -4.5)");

    verify_godot_side_read(
        Color::from_rgba(0.25, 0.5, 0.75, 0.125),
        "(0.25, 0.5, 0.75, 0.125)",
    );
    verify_godot_side_read(Quaternion::new(0.0, 0.0, 0.0, 1.0), "(0, 0, 0, 1)");
    verify_godot_side_read(
        Rect2i::new(Vector2i::new(1, 2), Vector2i::new(30, 40)),
        "[P: (1, 2), S: (30, 40)]",
    );
    verify_godot_side_read(
        Rect2::new(Vector2::new(1.5, 2.5), Vector2::new(30.5, 40.5)),
        "[P: (1.5, 2.5), S: (30.5, 40.5)]",
    );
    verify_godot_side_read(
        Plane::new(Vector3::new(0.26726124, 0.5345225, 0.80178374), 2.5),
        "[N: (0.267261, 0.534522, 0.801784), D: 2.5]",
    );

    // Rid's Rust representation is a niche-optimized enum, not a plain u64 -- worth an explicit Godot-side check.
    verify_godot_side_read(Rid::new(12345), "RID(12345)");
    verify_godot_side_read(Rid::Invalid, "RID(0)");
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Cross-FFI Layout Tests

/// Verify Rust-created (RustMarshal) and Godot-created (FFI) variants are byte-compatible.
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

/// Verify Rust-written variants are readable by Godot (reverse direction).
#[itest]
fn rust_variant_readable_by_godot() {
    let rust_variant = 12345_i64.to_variant();

    let mut arr: Array<Variant> = Array::new();
    arr.push(&rust_variant);

    // Godot reads back from its own storage.
    let retrieved = arr.at(0);
    assert_eq!(i64::from_variant(&retrieved), 12345_i64);

    // Godot-side stringify()/`==` read the payload independently, catching a bad offset that a round-trip alone would miss.
    assert_eq!(rust_variant.stringify().to_string(), "12345");
    assert_eq!(rust_variant, 12345_i64.to_variant());
}

// Engine-produced nil (via FFI) must match our zeroed-buffer `Variant::nil()`.
#[itest]
fn rust_variant_nil_matches_godot_nil() {
    let godot_nil = Callable::invalid().call(&[]);

    assert_eq!(godot_nil, Variant::nil());
}
