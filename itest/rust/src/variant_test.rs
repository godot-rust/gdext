use crate::itest;
use gdext_builtin::{FromVariant, GodotString, ToVariant, Variant};
use std::fmt::Debug;

pub fn run() -> bool {
    let mut ok = true;
    ok &= variant_conversions();
    ok &= variant_forbidden_conversions();
    ok &= variant_display();
    ok
}

#[itest]
fn variant_conversions() {
    roundtrip(false);
    roundtrip(true);
    roundtrip(GodotString::from("some string"));

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

fn roundtrip<T>(value: T)
where
    T: FromVariant + ToVariant + Debug + PartialEq + Clone,
{
    let variant = value.to_variant();
    let back = T::try_from_variant(&variant).unwrap();

    assert_eq!(value, back);
}

fn truncate_bad<T>(original_value: i64)
where
    T: FromVariant,
{
    let variant = original_value.to_variant();
    let result = T::try_from_variant(variant);

    result.expect_err();
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
