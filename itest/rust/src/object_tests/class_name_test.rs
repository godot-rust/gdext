/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::borrow::Cow;

use godot::builtin::{GString, StringName};
use godot::meta::ClassName;
use godot::obj::bounds::implement_godot_bounds;
use godot::obj::GodotClass;
use godot::sys;

use crate::framework::itest;

struct A;
struct U;

implement_godot_bounds!(A);
implement_godot_bounds!(U);

impl GodotClass for A {
    type Base = godot::classes::Object;

    fn class_name() -> ClassName {
        ClassName::new_cached::<A>(|| "A".to_string())
    }
}

impl GodotClass for U {
    type Base = godot::classes::Object;

    fn class_name() -> ClassName {
        ClassName::new_cached::<U>(|| "统一码".to_string())
    }
}

#[itest]
fn class_name_dynamic() {
    let a = A::class_name();
    let b = A::class_name();

    assert_eq!(a, b);
    assert_eq!(sys::hash_value(&a), sys::hash_value(&b));

    assert_eq!(a.to_string(), "A");
    assert_eq!(a.to_gstring(), GString::from("A"));
    assert_eq!(a.to_string_name(), StringName::from("A"));
    assert_eq!(a.to_cow_str(), Cow::<'static, str>::Owned("A".to_string()));
}

#[cfg(since_api = "4.4")]
#[itest]
fn class_name_dynamic_unicode() {
    let a = U::class_name();
    let b = U::class_name();

    assert_eq!(a, b);
    assert_eq!(sys::hash_value(&a), sys::hash_value(&b));

    assert_eq!(a.to_string(), "统一码");
    assert_eq!(a.to_gstring(), GString::from("统一码"));
    assert_eq!(a.to_string_name(), StringName::from("统一码"));
    assert_eq!(
        a.to_cow_str(),
        Cow::<'static, str>::Owned("统一码".to_string())
    );
}

// Test Unicode proc-macro support for ClassName.
#[cfg(since_api = "4.4")]
#[derive(godot::register::GodotClass)]
#[class(no_init)]
struct 统一码 {}
