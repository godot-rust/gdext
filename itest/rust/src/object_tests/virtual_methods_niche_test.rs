/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

#![cfg(feature = "codegen-full")]

use godot::classes::resource_loader::CacheMode;
use godot::classes::{
    BoxMesh, IResourceFormatLoader, IRigidBody2D, InputEvent, InputEventAction,
    ResourceFormatLoader, ResourceLoader, Viewport, Window,
};
use godot::obj::Singleton;
use godot::prelude::*;

use crate::framework::{itest, TestContext};
use crate::object_tests::virtual_methods_test::VirtualInputTest;

#[derive(GodotClass, Debug)]
#[class(init, base=ResourceFormatLoader)]
struct FormatLoaderTest {
    base: Base<ResourceFormatLoader>,
}

impl FormatLoaderTest {
    fn resource_type() -> GString {
        GString::from("some_resource_type")
    }
}

#[godot_api]
impl IResourceFormatLoader for FormatLoaderTest {
    fn get_recognized_extensions(&self) -> PackedStringArray {
        [GString::from("extension")].into_iter().collect()
    }

    fn handles_type(&self, type_: StringName) -> bool {
        type_.to_string() == Self::resource_type().to_string()
    }

    fn get_resource_type(&self, _path: GString) -> GString {
        Self::resource_type()
    }

    fn exists(&self, _path: GString) -> bool {
        true
    }

    fn load(
        &self,
        _path: GString,
        _original_path: GString,
        _use_sub_threads: bool,
        _cache_mode: i32,
    ) -> Variant {
        BoxMesh::new_gd().to_variant()
    }
}

// ------------------------------------------------------------------------------------------------------------------------------------------

// Used in `test_collision_object_2d_input_event` in `SpecialTests.gd`.
#[derive(GodotClass)]
#[class(init, base = RigidBody2D)]
pub struct CollisionObject2DTest {
    input_event_called: bool,
    viewport: Option<Gd<Viewport>>,
}

#[godot_api]
impl IRigidBody2D for CollisionObject2DTest {
    fn input_event(&mut self, viewport: Gd<Viewport>, _event: Gd<InputEvent>, _shape_idx: i32) {
        self.input_event_called = true;
        self.viewport = Some(viewport);
    }
}

#[godot_api]
impl CollisionObject2DTest {
    #[func]
    fn input_event_called(&self) -> bool {
        self.input_event_called
    }

    #[func]
    fn get_viewport(&self) -> Variant {
        self.viewport
            .as_ref()
            .map(ToGodot::to_variant)
            .unwrap_or(Variant::nil())
    }
}

// ------------------------------------------------------------------------------------------------------------------------------------------

#[itest]
fn test_format_loader(_test_context: &TestContext) {
    let format_loader = FormatLoaderTest::new_gd();

    let mut loader = ResourceLoader::singleton();
    loader
        .add_resource_format_loader_ex(&format_loader)
        .at_front(true)
        .done();

    let mut extensions_rust = format_loader.bind().get_recognized_extensions();
    extensions_rust.push("tres");

    let extensions = loader.get_recognized_extensions_for_type(&FormatLoaderTest::resource_type());
    assert_eq!(extensions, extensions_rust);

    let resource = loader
        .load_ex("path.extension")
        .cache_mode(CacheMode::IGNORE)
        .done()
        .unwrap();
    assert!(resource.try_cast::<BoxMesh>().is_ok());

    loader.remove_resource_format_loader(&format_loader);
}

#[itest]
fn test_input_event(test_context: &TestContext) {
    let obj = VirtualInputTest::new_alloc();
    assert_eq!(obj.bind().event, None);
    let mut test_viewport = Window::new_alloc();

    test_context.scene_tree.clone().add_child(&test_viewport);

    test_viewport.add_child(&obj);

    let mut event = InputEventAction::new_gd();
    event.set_action("debug");
    event.set_pressed(true);

    // We're running in headless mode, so Input.parse_input_event does not work
    test_viewport.clone().push_input(&event);

    assert_eq!(obj.bind().event, Some(event.upcast::<InputEvent>()));

    test_viewport.queue_free();
}

// We were incrementing/decrementing the refcount wrong. Which only showed up if you had multiple virtual
// methods handle the same refcounted object. Related to https://github.com/godot-rust/gdext/issues/257.
#[itest]
fn test_input_event_multiple(test_context: &TestContext) {
    let mut objs = Vec::new();
    for _ in 0..5 {
        let obj = VirtualInputTest::new_alloc();
        assert_eq!(obj.bind().event, None);
        objs.push(obj);
    }
    let mut test_viewport = Window::new_alloc();

    test_context.scene_tree.clone().add_child(&test_viewport);

    for obj in objs.iter() {
        test_viewport.add_child(obj)
    }

    let mut event = InputEventAction::new_gd();
    event.set_action("debug");
    event.set_pressed(true);

    // We're running in headless mode, so Input.parse_input_event does not work
    test_viewport.push_input(&event);

    for obj in objs.iter() {
        assert_eq!(obj.bind().event, Some(event.clone().upcast::<InputEvent>()));
    }

    test_viewport.queue_free();
}
