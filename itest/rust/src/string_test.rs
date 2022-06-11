use crate::godot_itest;
use gdext_builtin::{GodotString, StringName};

// TODO use tests from godot-rust/gdnative

pub fn run() -> bool {
    let mut ok = true;
    ok &= string_conversion();
    ok &= string_equality();
    ok &= string_ordering();
    ok &= string_clone();
    ok &= string_name_conversion();
    ok &= string_name_default_construct();
    ok
}

godot_itest! { string_conversion {
    let string = String::from("some string");
    let second = GodotString::from(&string);
    let back = String::from(&second);

    assert_eq!(string, back);
}}

godot_itest! { string_equality {
    let string = GodotString::from("some string");
    let second = GodotString::from("some string");
    let different = GodotString::from("some");

    assert_eq!(string, second);
    assert_ne!(string, different);
}}

godot_itest! { string_ordering {
    let low = GodotString::from("Alpha");
    let high = GodotString::from("Beta");

    assert!(low < high);
    assert!(low <= high);
    assert!(high > low);
    assert!(high >= low);
}}

godot_itest! { string_clone {
    let first = GodotString::from("some string");
    let cloned = first.clone();

    assert_eq!(first, cloned);
}}

// ----------------------------------------------------------------------------------------------------------------------------------------------

godot_itest! { string_name_conversion {
    let string = GodotString::from("some string");
    let name = StringName::from(&string);
    let back = GodotString::from(&name);

    assert_eq!(string, back);
}}

godot_itest! { string_name_default_construct {
    let name = StringName::default();
    let back = GodotString::from(&name);

    assert_eq!(back, GodotString::new());
}}
