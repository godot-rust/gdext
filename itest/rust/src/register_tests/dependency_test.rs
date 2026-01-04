/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::prelude::*;
use itest_dependency::DependencyObj;

use crate::framework::itest;

#[itest]
fn test_dependent_object_method() {
    let mut obj = DependencyObj::new_gd();
    // Route call through Godot to see if method has been registered properly.
    let num: i64 = obj.call("method_from_dependency", &[]).to();
    assert_eq!(num, 42);
}

#[itest]
fn test_dependent_object_property() {
    let obj = DependencyObj::new_gd();
    // Get property through Godot to see if it has been registered properly.
    let num: i64 = obj.get("some_property").to();
    assert_eq!(num, 42);
}
