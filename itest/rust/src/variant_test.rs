use crate::godot_itest;
use gdext_builtin::Variant;

pub fn run() -> bool {
    let mut ok = true;
    ok &= variant_tests();
    ok
}

godot_itest! { variant_tests {
    let v = Variant::nil();
    assert_eq!(v.to_string(), "null");
}}
