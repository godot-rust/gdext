/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::classes::Node;
use godot::prelude::*;
use godot::tools::{get_autoload_by_name, try_get_autoload_by_name};

use crate::framework::itest;

#[derive(GodotClass)]
#[class(init, base=Node)]
struct AutoloadClass {
    base: Base<Node>,
    #[var]
    property: i32,
}

#[godot_api]
impl AutoloadClass {
    #[func]
    fn verify_works(&self) -> i32 {
        787
    }
}

#[itest]
fn autoload_get() {
    let mut autoload = get_autoload_by_name::<AutoloadClass>("MyAutoload");
    {
        let mut guard = autoload.bind_mut();
        assert_eq!(guard.verify_works(), 787);
        assert_eq!(guard.property, 0, "still has default value");

        guard.property = 42;
    }

    // Fetch same autoload anew.
    let autoload2 = get_autoload_by_name::<AutoloadClass>("MyAutoload");
    assert_eq!(autoload2.bind().property, 42);

    // Reset for other tests.
    autoload.bind_mut().property = 0;
}

#[itest]
fn autoload_try_get_named() {
    let autoload = try_get_autoload_by_name::<AutoloadClass>("MyAutoload").expect("fetch autoload");

    assert_eq!(autoload.bind().verify_works(), 787);
    assert_eq!(autoload.bind().property, 0, "still has default value");
}

#[itest]
fn autoload_try_get_named_inexistent() {
    let result = try_get_autoload_by_name::<AutoloadClass>("InexistentAutoload");
    result.expect_err("non-existent autoload");
}

#[itest]
fn autoload_try_get_named_bad_type() {
    let result = try_get_autoload_by_name::<Node2D>("MyAutoload");
    result.expect_err("autoload of incompatible node type");
}
