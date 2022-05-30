use crate::godot_itest;
use gdext_builtin::{GodotString, Variant};

pub fn run() -> bool {
    let mut ok = true;
    ok &= variant_display();
    ok
}

// fn variant_conversions() {
//     (Variant::from(18446744073709551615u64), "18446744073709551615"),
//
// }

godot_itest! { variant_display {
    let cases = [
        (Variant::nil(), "null"),
        (Variant::from(false), "false"),
        (Variant::from(true), "true"),
        (Variant::from(GodotString::from("some string")), "some string"),

        // unsigned
        (Variant::from(0u8), "0"),
        (Variant::from(255u8), "255"),
        (Variant::from(0u16), "0"),
        (Variant::from(65535u16), "65535"),
        (Variant::from(0u32), "0"),
        (Variant::from(4294967295u32), "4294967295"),

        // signed
        (Variant::from(127i8), "127"),
        (Variant::from(-128i8), "-128"),
        (Variant::from(32767i16), "32767"),
        (Variant::from(-32768i16), "-32768"),
        (Variant::from(2147483647i32), "2147483647"),
        (Variant::from(-2147483648i32), "-2147483648"),
    ];

    for (variant, string) in cases {
        assert_eq!(&variant.to_string(), string);
    }
}}
