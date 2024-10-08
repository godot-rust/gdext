/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// This file tests the presence, naming and accessibility of generated symbols.
// Functionality is only tested on a superficial level (to make sure general FFI mechanisms work).

use crate::framework::itest;
use godot::builtin::inner::{InnerColor, InnerString};
use godot::classes::{FileAccess, HttpRequest, IHttpRequest, Image};
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

    let obj = Gd::from_init_fn(|base| CodegenTest { _base: base });
    let _id = obj.instance_id();

    obj.free();
}

#[itest]
fn codegen_static_builtin_method() {
    let pi = InnerString::num(std::f64::consts::PI, 3);
    assert_eq!(pi, GString::from("3.142"));

    let col = InnerColor::html("#663399cc");
    assert_eq!(col, Color::from_rgba(0.4, 0.2, 0.6, 0.8));
}

#[itest]
fn codegen_static_class_method() {
    let exists = FileAccess::file_exists("inexistent");
    assert!(!exists);

    let exists = FileAccess::file_exists("res://itest.gdextension");
    assert!(exists);

    // see also object_test for reference count verification
}

#[itest]
fn codegen_constants() {
    assert_eq!(Image::MAX_WIDTH, 16777216);
    // assert_eq!(Material::RENDER_PRIORITY_MIN, -128);
}

#[itest]
fn cfg_test() {
    // Makes sure that since_api and before_api are mutually exclusive
    assert_ne!(cfg!(since_api = "4.2"), cfg!(before_api = "4.2"));
    assert_ne!(cfg!(since_api = "4.3"), cfg!(before_api = "4.3"));
}

#[derive(GodotClass)]
#[class(base=HttpRequest)] // test a base class that is renamed in Godot
pub struct CodegenTest {
    _base: Base<HttpRequest>,
}

#[allow(unused)]
#[godot_api]
impl CodegenTest {
    #[func]
    fn with_unnamed(&self, _: i32) {}

    #[func]
    fn with_unused(&self, _unused: i32) {}

    #[func]
    fn with_mut(&self, mut param: i32) {}

    #[func]
    fn with_many_unnamed(&self, _: i32, _: GString) {}
}

#[godot_api]
impl IHttpRequest for CodegenTest {
    fn init(base: Base<HttpRequest>) -> Self {
        CodegenTest { _base: base }
    }

    // Test unnamed parameter in virtual function
    fn process(&mut self, _: f64) {}
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[cfg(since_api = "4.3")]
#[derive(GodotClass)]
#[class(no_init)]
pub struct CodegenTest2 {
    _base: Base<RefCounted>,
}

#[cfg(since_api = "4.3")]
#[godot_api]
#[allow(unused)]
impl CodegenTest2 {
    #[func(virtual)]
    fn with_virtual_unnamed(&self, _: i32) {}

    #[cfg(since_api = "4.3")]
    #[func(virtual, gd_self)]
    fn with_virtual_unnamed_gdself(this: Gd<Self>, _: i32) {}

    #[cfg(since_api = "4.3")]
    #[func(virtual)]
    fn with_virtual_unused(&self, _unused: i32) {}

    #[cfg(since_api = "4.3")]
    #[func(virtual)]
    fn with_virtual_mut(&self, mut param: i32) {}

    #[cfg(since_api = "4.3")]
    #[func(virtual)]
    fn with_virtual_many_unnamed(&self, _: i32, _: GString) {}
}
