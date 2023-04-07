/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// This file tests the presence, naming and accessibility of generated symbols.
// Functionality is only tested on a superficial level (to make sure general FFI mechanisms work).

use crate::itest;
use godot::builtin::inner::{InnerColor, InnerString};
use godot::engine::{FileAccess, HttpRequest, HttpRequestVirtual, Image};
use godot::prelude::*;

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

#[itest]
fn codegen_static_builtin_method() {
    let pi = InnerString::num(std::f64::consts::PI, 3);
    assert_eq!(pi, GodotString::from("3.142"));

    let col = InnerColor::html("#663399cc".into());
    assert_eq!(col, Color::from_rgba(0.4, 0.2, 0.6, 0.8));
}

#[itest]
fn codegen_static_class_method() {
    let exists = FileAccess::file_exists("inexistent".into());
    assert!(!exists);

    let exists = FileAccess::file_exists("res://itest.gdextension".into());
    assert!(exists);

    // see also object_test for reference count verification
}

#[itest]
fn codegen_constants() {
    assert_eq!(Image::MAX_WIDTH, 16777216);
    // assert_eq!(Material::RENDER_PRIORITY_MIN, -128);
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[derive(GodotClass)]
#[class(base=HttpRequest)]
pub struct TestBaseRenamed {
    #[base]
    base: Base<HttpRequest>,
}

#[godot_api]
impl HttpRequestVirtual for TestBaseRenamed {
    fn init(base: Base<HttpRequest>) -> Self {
        TestBaseRenamed { base }
    }
}
