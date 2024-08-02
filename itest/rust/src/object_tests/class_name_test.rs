/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
use crate::framework::itest;
use godot::builtin::{GString, StringName};
use godot::meta::ClassName;
use godot::obj::bounds::implement_godot_bounds;
use godot::obj::GodotClass;
use godot::sys;
use std::borrow::Cow;

struct A;

implement_godot_bounds!(A);

impl GodotClass for A {
    type Base = godot::classes::Object;

    fn class_name() -> ClassName {
        ClassName::new_cached::<A>(|| "A".to_string())
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
