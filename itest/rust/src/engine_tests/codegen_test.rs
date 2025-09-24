/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// This file tests the presence, naming and accessibility of generated symbols.
// Functionality is only tested on a superficial level (to make sure general FFI mechanisms work).

use godot::builtin::inner::InnerColor;
use godot::classes::{FileAccess, HttpRequest, IHttpRequest, RenderingServer};
use godot::prelude::*;

use crate::framework::itest;

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
    let pi = GString::num(std::f64::consts::PI, 3);
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
    assert_eq!(RenderingServer::CANVAS_ITEM_Z_MIN, -4096);
    //assert_eq!(Image::MAX_WIDTH, 16777216);
    // assert_eq!(Material::RENDER_PRIORITY_MIN, -128);
}

#[itest]
fn cfg_test() {
    // Makes sure that since_api and before_api are mutually exclusive.
    assert_ne!(cfg!(since_api = "4.3"), cfg!(before_api = "4.3"));
    assert_ne!(cfg!(since_api = "4.4"), cfg!(before_api = "4.4"));
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

    // Test auto-cast to f32 parameter in virtual function
    fn physics_process(&mut self, delta: f32) {
        // Test it's actually f32 in the body.
        let _use_param: f32 = delta;
    }
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

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Generation of APIs via declarative macro.

macro_rules! make_class {
    ($ClassName:ident, $BaseName:ident) => {
        #[derive(GodotClass)]
        #[class(base=$BaseName)]
        pub struct $ClassName {
            base: Base<godot::classes::$BaseName>,
        }
    };
}

macro_rules! make_interface_impl {
    ($Class:ty, $Trait:path) => {
        #[godot_api]
        #[allow(unused)]
        impl $Trait for $Class {
            fn init(base: Base<Self::Base>) -> Self {
                Self { base }
            }

            fn exit_tree(&mut self) {}
        }
    };
}

macro_rules! make_user_api {
    ($Class:ty, $method:ident, $Param:ty) => {
        #[godot_api]
        #[allow(unused)]
        impl $Class {
            #[func]
            fn $method(&self, _m: $Param) {}
        }
    };
}

make_class!(CodegenTest3, Node3D);
make_interface_impl!(CodegenTest3, INode3D);
make_user_api!(CodegenTest3, take_param, i32);

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Regression tests for ambiguous method calls: https://github.com/godot-rust/gdext/issues/858
// Also references the above macro-generated class.

#[allow(dead_code)]
trait TraitA {
    fn exit_tree(&mut self);
}

impl TraitA for CodegenTest3 {
    fn exit_tree(&mut self) {}
}

// Verifies that attributes (here #[expect]) are preserved by #[itest] macro.
// See retain_attributes_except() function.
#[itest]
#[expect(unused_variables)]
fn itest_macro_attribute_retention() {
    let unused_var = 42; // Should not generate warning.
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Int->enum replacements

// Tests both that code compiles, and that FFI does not break by replacement.
#[cfg(feature = "codegen-full")]
#[itest]
fn changed_enum_apis() {
    use godot::classes::file_access::ModeFlags;
    use godot::classes::gpu_particles_2d::EmitFlags;
    use godot::classes::tree::DropModeFlags;
    use godot::classes::{FileAccess, GpuParticles2D, Tree};

    use crate::framework::suppress_godot_print;

    suppress_godot_print(|| {
        // FileAccess::create_temp() with ModeFlags. Deliberately invalid prefix to not create an actual file.
        let file = FileAccess::create_temp_ex(ModeFlags::READ)
            .prefix("/invalid-prefix")
            .done();
        assert!(file.is_none());
    });

    // GPUParticles2D::emit_particle with EmitFlags.
    let mut particles2d = GpuParticles2D::new_alloc();
    particles2d.emit_particle(
        Transform2D::IDENTITY,
        Vector2::ZERO,
        Color::RED,
        Color::BLACK,
        EmitFlags::POSITION | EmitFlags::ROTATION_SCALE,
    );
    particles2d.free();

    // Tree::{set,get}_drop_mode_flags() with DropModeFlags.
    let mut tree = Tree::new_alloc();
    tree.set_drop_mode_flags(DropModeFlags::INBETWEEN);
    let mode = tree.get_drop_mode_flags();
    assert_eq!(mode, DropModeFlags::INBETWEEN);
    tree.free();
}
