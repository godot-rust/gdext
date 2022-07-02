use crate::itest;
use gdext_builtin::{GodotString, Variant};
use std::fmt::Debug;

pub fn run() -> bool {
    let mut ok = true;
    ok &= variant_conversions();
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

fn roundtrip<T>(value: T)
where
    for<'a> T: From<&'a Variant> + Debug + PartialEq + Clone, // TODO use From<Variant>
    Variant: From<T>,
{
    let variant = Variant::from(value.clone());
    let back = T::from(&variant);

    assert_eq!(value, back);
}

#[itest]
fn variant_display() {
    let cases = [
        (Variant::nil(), "null"),
        (Variant::from(false), "false"),
        (Variant::from(true), "true"),
        (
            Variant::from(GodotString::from("some string")),
            "some string",
        ),
        //
        // unsigned
        (Variant::from(0u8), "0"),
        (Variant::from(255u8), "255"),
        (Variant::from(0u16), "0"),
        (Variant::from(65535u16), "65535"),
        (Variant::from(0u32), "0"),
        (Variant::from(4294967295u32), "4294967295"),
        //
        // signed
        (Variant::from(127i8), "127"),
        (Variant::from(-128i8), "-128"),
        (Variant::from(32767i16), "32767"),
        (Variant::from(-32768i16), "-32768"),
        (Variant::from(2147483647i32), "2147483647"),
        (Variant::from(-2147483648i32), "-2147483648"),
        (Variant::from(9223372036854775807i64), "9223372036854775807"),
        (
            Variant::from(-9223372036854775808i64),
            "-9223372036854775808",
        ),
    ];

    for (variant, string) in cases {
        assert_eq!(&variant.to_string(), string);
    }
}
