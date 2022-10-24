/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::itest;
use godot_core::builtin::{FromVariant, GodotString, ToVariant, Variant, Vector2, Vector3};
use godot_core::obj::InstanceId;
use godot_ffi::{VariantOperator, VariantType};
use std::fmt::{Debug, Display};

pub fn run() -> bool {
    let mut ok = true;
    ok &= variant_nil();
    ok &= variant_conversions();
    ok &= variant_forbidden_conversions();
    ok &= variant_display();
    ok &= variant_get_type();
    ok &= variant_equal();
    ok &= variant_evaluate();
    ok
}

#[itest]
fn variant_nil() {
    let variant = Variant::nil();
    assert!(variant.is_nil());

    let variant = 12345i32.to_variant();
    assert!(!variant.is_nil());
}

#[itest]
fn variant_conversions() {
    roundtrip(false);
    roundtrip(true);
    roundtrip(GodotString::from("some string"));
    roundtrip(InstanceId::from_nonzero(-9223372036854775808i64));
    // roundtrip(Some(InstanceId::from_nonzero(9223372036854775807i64)));
    // roundtrip(Option::<InstanceId>::None);

    // unsigned
    roundtrip(0u8);
    roundtrip(255u8);
    roundtrip(0u16);
    roundtrip(65535u16);
    roundtrip(0u32);
    roundtrip(4294967295u32);

    // signed
    roundtrip(127i8);
    roundtrip(-128i8);
    roundtrip(32767i16);
    roundtrip(-32768i16);
    roundtrip(2147483647i32);
    roundtrip(-2147483648i32);
    roundtrip(9223372036854775807i64);
}

#[itest]
fn variant_forbidden_conversions() {
    truncate_bad::<i8>(128);
}

#[itest]
fn variant_get_type() {
    let variant = Variant::nil();
    assert_eq!(variant.get_type(), VariantType::Nil);

    let variant = 74i32.to_variant();
    assert_eq!(variant.get_type(), VariantType::Int);

    let variant = true.to_variant();
    assert_eq!(variant.get_type(), VariantType::Bool);

    let variant = GodotString::from("hello").to_variant();
    assert_eq!(variant.get_type(), VariantType::String);
}

#[itest]
fn variant_equal() {
    assert_eq!(Variant::nil(), ().to_variant());
    assert_eq!(Variant::from(77), 77.to_variant());

    equal(77, (), false);
    equal(77, 78, false);

    assert_ne!(77.to_variant(), Variant::nil());
    assert_ne!(77.to_variant(), 78.to_variant());

    //equal(77, 77.0, false)
    equal(Vector3::new(1.0, 2.0, 3.0), Vector2::new(1.0, 2.0), false);
    equal(1, true, false);
    equal(false, 0, false);
    equal(GodotString::from("String"), 33, false);
}

#[itest]
fn variant_evaluate() {
    evaluate(VariantOperator::Add, 20, -39, -19);
    evaluate(VariantOperator::Greater, 20, 19, true);
    evaluate(VariantOperator::Equal, 20, 20.0, true);
    evaluate(VariantOperator::NotEqual, 20, 20.0, false);
    evaluate(VariantOperator::Multiply, 5, 2.5, 12.5);

    evaluate_fail(VariantOperator::Equal, 1, true);
    evaluate_fail(VariantOperator::Equal, 0, false);
    evaluate_fail(VariantOperator::Subtract, 2, Vector3::new(1.0, 2.0, 3.0));
}

#[itest]
fn variant_display() {
    let cases = [
        (Variant::nil(), "<null>"),
        (false.to_variant(), "false"),
        (true.to_variant(), "true"),
        (GodotString::from("some string").to_variant(), "some string"),
        //
        // unsigned
        ((0u8).to_variant(), "0"),
        ((255u8).to_variant(), "255"),
        ((0u16).to_variant(), "0"),
        ((65535u16).to_variant(), "65535"),
        ((0u32).to_variant(), "0"),
        ((4294967295u32).to_variant(), "4294967295"),
        //
        // signed
        ((127i8).to_variant(), "127"),
        ((-128i8).to_variant(), "-128"),
        ((32767i16).to_variant(), "32767"),
        ((-32768i16).to_variant(), "-32768"),
        ((2147483647i32).to_variant(), "2147483647"),
        ((-2147483648i32).to_variant(), "-2147483648"),
        ((9223372036854775807i64).to_variant(), "9223372036854775807"),
        (
            (-9223372036854775808i64).to_variant(),
            "-9223372036854775808",
        ),
    ];

    for (variant, string) in cases {
        assert_eq!(&variant.to_string(), string);
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

fn roundtrip<T>(value: T)
where
    T: FromVariant + ToVariant + PartialEq + Debug,
{
    // TODO test other roundtrip (first FromVariant, then ToVariant)
    // Some values can be represented in Variant, but not in T (e.g. Variant(0i64) -> Option<InstanceId> -> Variant is lossy)

    let variant = value.to_variant();
    let back = T::try_from_variant(&variant).unwrap();

    assert_eq!(value, back);
}

fn truncate_bad<T>(original_value: i64)
where
    T: FromVariant + Display,
{
    let variant = original_value.to_variant();
    let result = T::try_from_variant(&variant);

    if let Ok(back) = result {
        panic!(
            "{} - T::try_from_variant({}) should fail, but resulted in {}",
            std::any::type_name::<T>(),
            variant,
            back
        );
    }
}

fn equal<T, U>(lhs: T, rhs: U, expected: bool)
where
    T: ToVariant,
    U: ToVariant,
{
    if expected {
        assert_eq!(lhs.to_variant(), rhs.to_variant());
    } else {
        assert_ne!(lhs.to_variant(), rhs.to_variant());
    }
}

fn evaluate<T, U, E>(op: VariantOperator, lhs: T, rhs: U, expected: E)
where
    T: ToVariant,
    U: ToVariant,
    E: ToVariant,
{
    let lhs = lhs.to_variant();
    let rhs = rhs.to_variant();
    let expected = expected.to_variant();

    assert_eq!(lhs.evaluate(&rhs, op), Some(expected));
}

fn evaluate_fail<T, U>(op: VariantOperator, lhs: T, rhs: U)
where
    T: ToVariant,
    U: ToVariant,
{
    let lhs = lhs.to_variant();
    let rhs = rhs.to_variant();

    assert_eq!(lhs.evaluate(&rhs, op), None);
}
