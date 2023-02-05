/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// This file tests the presence and naming of generated symbols, not their functionality.

use crate::itest;

use godot::engine::HttpRequest;
use godot::prelude::*;

pub fn run() -> bool {
    let mut ok = true;
    ok &= codegen_class_renamed();
    ok &= codegen_base_renamed();
    ok
}

#[itest]
fn codegen_class_renamed() {
    // Known as `HTTPRequest` in Godot
    let obj = HttpRequest::new_alloc();
    obj.free();
}

#[itest]
fn codegen_base_renamed() {
    // The registration is done at startup time, so it may already fail during GDExtension init.
    // Nevertheless, try to instantiate an object with base HttpRequest here.

    let obj = Gd::with_base(|base| TestBaseRenamed { base });
    let _id = obj.instance_id();

    obj.free();
}

#[derive(GodotClass)]
#[class(base=HttpRequest)]
pub struct TestBaseRenamed {
    #[base]
    base: Base<HttpRequest>,
}

#[godot_api]
impl GodotExt for TestBaseRenamed {
    fn init(base: Base<HttpRequest>) -> Self {
        TestBaseRenamed { base }
    }
}
