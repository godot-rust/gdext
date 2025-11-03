/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::borrow::Cow;

use godot::builtin::{GString, StringName};
use godot::meta::ClassId;
use godot::obj::bounds::implement_godot_bounds;
use godot::obj::GodotClass;
use godot::sys;

use crate::framework::{expect_panic, itest};

struct A;
struct U;

implement_godot_bounds!(A);
implement_godot_bounds!(U);

impl GodotClass for A {
    type Base = godot::classes::Object;

    fn class_id() -> ClassId {
        ClassId::new_cached::<A>(|| "A".to_string())
    }
}

impl GodotClass for U {
    type Base = godot::classes::Object;

    fn class_id() -> ClassId {
        ClassId::new_cached::<U>(|| "统一码".to_string())
    }
}

#[itest]
fn class_name_godotclass() {
    let a = A::class_id();
    let b = A::class_id();

    assert_eq!(a, b);
    assert_eq!(sys::hash_value(&a), sys::hash_value(&b));

    assert_eq!(a.to_string(), "A");
    assert_eq!(a.to_gstring(), GString::from("A"));
    assert_eq!(a.to_string_name(), StringName::from("A"));
    assert_eq!(a.to_cow_str(), Cow::<'static, str>::Owned("A".to_string()));
}

#[cfg(since_api = "4.4")]
#[itest]
fn class_name_godotclass_unicode() {
    let a = U::class_id();
    let b = U::class_id();

    assert_eq!(a, b);
    assert_eq!(sys::hash_value(&a), sys::hash_value(&b));

    assert_eq!(a.to_string(), "统一码");
    assert_eq!(a.to_gstring(), GString::from("统一码"));
    assert_eq!(a.to_string_name(), StringName::from("统一码"));
    assert_eq!(
        a.to_cow_str(),
        Cow::<'static, str>::Owned("统一码".to_string())
    );

    let b = ClassId::__dynamic("统一码");
    assert_eq!(a, b);
}

#[itest]
fn class_name_from_dynamic() {
    // Test that runtime-constructed class names are equal to compile-time ones.
    let comptime = A::class_id();
    let runtime = ClassId::__dynamic("A");
    assert_eq!(comptime, runtime);
    assert_eq!(sys::hash_value(&comptime), sys::hash_value(&runtime));
    assert_eq!(comptime.to_string(), runtime.to_string());

    // Test that multiple runtime constructions of the same name are equal.
    let runtime2 = ClassId::__dynamic("A");
    assert_eq!(runtime, runtime2);

    // Test with a different name.
    let different_runtime = ClassId::__dynamic("B");
    assert_ne!(comptime, different_runtime);
    assert_eq!(different_runtime.to_string(), "B");
}

#[itest]
fn class_name_empty() {
    // Empty string and ClassName::none() should be the same.
    let none_dynamic = ClassId::__dynamic("");
    let none = ClassId::none();

    assert_eq!(none_dynamic, none);
    assert_eq!(format!("{none_dynamic:?}"), "ClassId(none)");
    assert_eq!(format!("{none:?}"), "ClassId(none)");
}

// Test Unicode proc-macro support for ClassName.
#[cfg(since_api = "4.4")]
#[derive(godot::register::GodotClass)]
#[class(no_init)]
struct 统一码 {}

#[itest]
fn class_name_dynamic_then_static() {
    struct A;

    // First, insert dynamic string, then static one.
    let dynamic_name = ClassId::__dynamic("LocalA");
    let static_name = ClassId::__cached::<A>(|| "LocalA".to_string());

    // They should be equal (same global_index), but current implementation may create duplicates
    assert_eq!(
        dynamic_name, static_name,
        "Dynamic and static ClassId for same string should be equal"
    );
}

#[itest]
fn class_name_static_then_dynamic() {
    struct B;

    // First, insert static string, then dynamic one.
    let static_name = ClassId::__cached::<B>(|| "LocalB".to_string());
    let dynamic_name = ClassId::__dynamic("LocalB");

    // They should be equal (same global_index)
    assert_eq!(
        static_name, dynamic_name,
        "Static and dynamic ClassId for same string should be equal"
    );
}

#[itest]
fn class_name_debug() {
    struct TestDebugClass;

    // Test debug output for various class names
    let none_name = ClassId::none();
    let dynamic_name = ClassId::__dynamic("MyDynamicClass");
    let static_name = ClassId::__cached::<TestDebugClass>(|| "MyStaticClass".to_string());

    // Verify debug representations include the actual class names
    assert_eq!(format!("{none_name:?}"), "ClassId(none)");
    assert_eq!(format!("{dynamic_name:?}"), "ClassId(\"MyDynamicClass\")");
    assert_eq!(format!("{static_name:?}"), "ClassId(\"MyStaticClass\")");
}

#[cfg(safeguards_balanced)]
#[itest]
fn class_name_alloc_panic() {
    // ASCII.
    {
        let _1st = ClassId::__alloc_next_unicode("DuplicateTestClass");

        expect_panic("2nd allocation with same ASCII string fails", || {
            let _2nd = ClassId::__alloc_next_unicode("DuplicateTestClass");
        });
    }

    // Unicode.
    #[cfg(since_api = "4.4")]
    {
        let _1st = ClassId::__alloc_next_unicode("クラス名");

        expect_panic("2nd allocation with same Unicode string fails", || {
            let _2nd = ClassId::__alloc_next_unicode("クラス名");
        });
    }
}
