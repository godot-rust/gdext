use crate::godot_itest;
use gdext_builtin::{GodotString, StringName};
use gdext_sys::{self as sys, interface_fn};

// TODO use tests from godot-rust/gdnative

pub fn run() -> bool {
    let mut ok = true;
    ok &= string_conversion();
    ok &= string_equality();
    ok &= string_ordering();
    ok &= string_clone();
    ok &= string_relocate();
    ok &= string_name_conversion();
    ok &= string_name_default_construct();
    ok &= string_name_relocate();
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

#[inline(never)]
fn make_string_ptr() -> sys::GDNativeStringPtr {
    let stack_allocated = GodotString::from("some string");
    stack_allocated.leak_string_sys()
}

godot_itest! { string_relocate {
    let ptr = make_string_ptr();
    let reconstructed = unsafe { GodotString::from_string_sys(ptr) };

    assert_eq!(reconstructed, GodotString::from("some string"));
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

fn make_string_name() -> StringName {
    StringName::from(&GodotString::from("some string"))
}

#[inline(never)]
fn make_string_name_ptr() -> sys::GDNativeStringNamePtr {
    let stack_allocated = make_string_name();
    stack_allocated.leak_string_sys()
}

godot_itest! { string_name_relocate {
    let ptr = make_string_name_ptr();
    let reconstructed = unsafe { StringName::from_string_sys(ptr) };
    let back = GodotString::from(&reconstructed);

    assert_eq!(back, GodotString::from("some string"));
}}

// fn string_relocated() {
//     let first = MaybeUninit::new();
//     let second = MaybeUninit::uninit();
//
//     unsafe {
//         let first_ptr = first.assume_init_ref().string_sys();
//         interface_fn!(string_to_utf8_chars)(first_ptr)
//     }
//     assert_eq!(string, back);
// }}
