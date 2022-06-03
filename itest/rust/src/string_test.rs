use crate::godot_itest;
use gdext_builtin::GodotString;

pub fn run() -> bool {
    let mut ok = true;
    ok &= string_operators();
    ok
}

// TODO use tests from godot-rust/gdnative here
godot_itest! { string_operators {
    let string = GodotString::from("some string");
    let second = GodotString::from("some string");
    let different = GodotString::from("some");

    assert_eq!(string, second);
    assert_ne!(string, different);
}}
