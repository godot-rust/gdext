use crate::godot_itest;
use gdext_builtin::GodotString;

// TODO use tests from godot-rust/gdnative

pub fn run() -> bool {
    let mut ok = true;
    ok &= string_equality();
    ok &= string_ordering();
    ok &= string_clone();
    ok
}

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