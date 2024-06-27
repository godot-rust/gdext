/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

#![allow(dead_code)]

use crate::framework::{expect_panic, itest, TestContext};

use godot::builtin::{
    real, varray, Color, GString, PackedByteArray, PackedColorArray, PackedFloat32Array,
    PackedInt32Array, PackedStringArray, PackedVector2Array, PackedVector3Array, RealConv,
    StringName, Variant, VariantArray, Vector2, Vector3,
};
use godot::classes::notify::NodeNotification;
use godot::classes::resource_loader::CacheMode;
use godot::classes::{
    BoxMesh, INode, INode2D, IPrimitiveMesh, IRefCounted, IResourceFormatLoader, IRigidBody2D,
    InputEvent, InputEventAction, Node, Node2D, PrimitiveMesh, RefCounted, ResourceFormatLoader,
    ResourceLoader, Viewport, Window,
};
use godot::meta::ToGodot;
use godot::obj::{Base, Gd, NewAlloc, NewGd};
use godot::private::class_macros::assert_eq_approx;
use godot::register::{godot_api, GodotClass};

/// Simple class, that deliberately has no constructor accessible from GDScript
#[derive(GodotClass, Debug)]
#[class(no_init, base=RefCounted)]
struct WithoutInit {
    some_base: Base<RefCounted>,
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[derive(GodotClass, Debug)]
#[class(init, base=RefCounted)]
struct VirtualMethodTest {
    some_base: Base<RefCounted>,

    integer: i32,
}

#[godot_api]
impl IRefCounted for VirtualMethodTest {
    fn to_string(&self) -> GString {
        format!("VirtualMethodTest[integer={}]", self.integer).into()
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[derive(GodotClass, Debug)]
#[class(base=Node2D)]
struct VirtualReadyTest {
    some_base: Base<Node2D>,
    implementation_value: i32,
    panics: bool,
}

#[godot_api]
impl INode2D for VirtualReadyTest {
    fn init(base: Base<Node2D>) -> Self {
        VirtualReadyTest {
            some_base: base,
            implementation_value: 0,
            panics: false,
        }
    }

    fn ready(&mut self) {
        if self.panics {
            panic!("a bit too ready");
        }

        self.implementation_value += 1;
    }

    #[cfg(any())]
    fn to_string(&self) -> GString {
        compile_error!("Removed by #[cfg]")
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[derive(GodotClass, Debug)]
#[class(base=Node2D)]
struct VirtualTreeTest {
    some_base: Base<Node2D>,
    tree_enters: i32,
    tree_exits: i32,
}

#[godot_api]
impl INode2D for VirtualTreeTest {
    fn init(base: Base<Node2D>) -> Self {
        VirtualTreeTest {
            some_base: base,
            tree_enters: 0,
            tree_exits: 0,
        }
    }

    fn enter_tree(&mut self) {
        self.tree_enters += 1;
    }

    fn exit_tree(&mut self) {
        self.tree_exits += 1;
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[derive(GodotClass, Debug)]
#[class(init, base=PrimitiveMesh)]
struct VirtualReturnTest {
    base: Base<PrimitiveMesh>,
}

#[godot_api]
impl IPrimitiveMesh for VirtualReturnTest {
    fn create_mesh_array(&self) -> VariantArray {
        varray![
            PackedVector3Array::from_iter([Vector3::LEFT]),
            PackedVector3Array::from_iter([Vector3::LEFT]),
            PackedFloat32Array::from_iter([0.0, 0.0, 0.0, 1.0]),
            PackedColorArray::from_iter([Color::from_rgb(1.0, 1.0, 1.0)]),
            PackedVector2Array::from_iter([Vector2::LEFT]),
            PackedVector2Array::from_iter([Vector2::LEFT]),
            PackedByteArray::from_iter([0, 1, 2, 3]),
            PackedByteArray::from_iter([0, 1, 2, 3]),
            PackedByteArray::from_iter([0, 1, 2, 3]),
            PackedByteArray::from_iter([0, 1, 2, 3]),
            PackedInt32Array::from_iter([0, 1, 2, 3]),
            PackedFloat32Array::from_iter([0.0, 1.0, 2.0, 3.0]),
            PackedInt32Array::from_iter([0]),
        ]
    }
}

#[derive(GodotClass, Debug)]
#[class(base=Node2D)]
struct VirtualInputTest {
    base: Base<Node2D>,
    event: Option<Gd<InputEvent>>,
}

#[godot_api]
impl INode2D for VirtualInputTest {
    fn init(base: Base<Node2D>) -> Self {
        VirtualInputTest { base, event: None }
    }

    fn input(&mut self, event: Gd<InputEvent>) {
        self.event = Some(event);
    }
}

#[derive(GodotClass, Debug)]
#[class(init, base=ResourceFormatLoader)]
struct FormatLoaderTest {
    base: Base<ResourceFormatLoader>,
}

impl FormatLoaderTest {
    fn resource_type() -> GString {
        GString::from("foo")
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

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[derive(Eq, PartialEq, Debug)]
enum ReceivedEvent {
    Notification(NodeNotification),
    Ready,
}

#[derive(GodotClass, Debug)]
#[class(base=Node, init)]
struct NotificationTest {
    base: Base<Node>,

    sequence: Vec<ReceivedEvent>,
}

#[godot_api]
impl INode for NotificationTest {
    fn on_notification(&mut self, what: NodeNotification) {
        self.sequence.push(ReceivedEvent::Notification(what));
    }

    fn ready(&mut self) {
        self.sequence.push(ReceivedEvent::Ready);
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[derive(GodotClass)]
#[class(init)]
struct GetTest {
    #[var]
    always_get_hello: i64,
    #[var]
    gettable: i64,

    get_called: std::cell::Cell<bool>,
}

#[godot_api]
impl IRefCounted for GetTest {
    fn get_property(&self, property: StringName) -> Option<Variant> {
        self.get_called.set(true);

        match String::from(property).as_str() {
            "always_get_hello" => Some("hello".to_variant()),
            "gettable" => Some(self.gettable.to_variant()),
            _ => None,
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[derive(GodotClass)]
#[class(init)]
struct SetTest {
    #[var]
    always_set_to_100: i64,
    #[var]
    settable: i64,

    set_called: bool,
}

#[godot_api]
impl IRefCounted for SetTest {
    fn set_property(&mut self, property: StringName, value: Variant) -> bool {
        self.set_called = true;

        match String::from(property).as_str() {
            "always_set_to_100" => {
                self.always_set_to_100 = 100;
                true
            }
            "settable" => {
                self.settable = value.to();
                true
            }
            _ => false,
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[derive(GodotClass)]
#[class(init)]
struct RevertTest {}

#[godot_api]
impl IRefCounted for RevertTest {
    fn property_get_revert(&self, property: StringName) -> Option<Variant> {
        use std::sync::atomic::AtomicUsize;

        static INC: AtomicUsize = AtomicUsize::new(0);

        match String::from(property).as_str() {
            "property_not_revert" => None,
            "property_do_revert" => Some(GString::from("hello!").to_variant()),
            // No UB or anything else like a crash or panic should happen when `property_can_revert` and `property_get_revert` return
            // inconsistent values, but in case something like that happens we should be able to detect it through this function.
            "property_changes" => {
                if INC.fetch_add(1, std::sync::atomic::Ordering::AcqRel) % 2 == 0 {
                    None
                } else {
                    Some(true.to_variant())
                }
            }
            _ => None,
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[itest]
fn test_to_string() {
    let _obj = VirtualMethodTest::new_gd();
}

#[itest]
fn test_ready(test_context: &TestContext) {
    let obj = VirtualReadyTest::new_alloc();
    assert_eq!(obj.bind().implementation_value, 0);

    // Add to scene tree.
    let mut test_node = test_context.scene_tree.clone();
    test_node.add_child(obj.clone().upcast());

    // _ready runs, increments implementation_value once.
    assert_eq!(obj.bind().implementation_value, 1);
}

#[itest(focus)]
fn test_ready_panic(test_context: &TestContext) {
    let mut obj = VirtualReadyTest::new_alloc();
    obj.bind_mut().panics = true;

    // Add to scene tree -- this panics.
    let mut test_node = test_context.scene_tree.clone();
    expect_panic("panic in ready() propagated to caller", || {
        test_node.add_child(obj.clone().upcast());
    });

    assert_eq!(obj.bind().implementation_value, 0);
}

#[itest(focus)]
fn test_ready_dynamic_panic(test_context: &TestContext) {
    let mut obj = VirtualReadyTest::new_alloc();
    obj.bind_mut().panics = true;

    // Add to scene tree -- this panics.
    let mut test_node = test_context.scene_tree.clone();

    // FIXME implement dynamic calls.
    let result = test_node.try_call("add_child".into(), &[obj.to_variant()]);
    let err = result.expect_err("add_child() should have panicked");
    dbg!(err);

    assert_eq!(obj.bind().implementation_value, 0);
}

#[itest]
fn test_ready_multiple_fires(test_context: &TestContext) {
    let obj = VirtualReadyTest::new_alloc();
    assert_eq!(obj.bind().implementation_value, 0);

    let mut test_node = test_context.scene_tree.clone();

    // Add to scene tree.
    test_node.add_child(obj.clone().upcast());

    // _ready runs, increments implementation_value once.
    assert_eq!(obj.bind().implementation_value, 1);

    // Remove and re-add to scene tree.
    test_node.remove_child(obj.clone().upcast());
    test_node.add_child(obj.clone().upcast());

    // _ready does NOT run again, implementation_value should still be 1.
    assert_eq!(obj.bind().implementation_value, 1);
}

#[itest]
fn test_ready_request_ready(test_context: &TestContext) {
    let obj = VirtualReadyTest::new_alloc();
    assert_eq!(obj.bind().implementation_value, 0);

    let mut test_node = test_context.scene_tree.clone();

    // Add to scene tree.
    test_node.add_child(obj.clone().upcast());

    // _ready runs, increments implementation_value once.
    assert_eq!(obj.bind().implementation_value, 1);

    // Remove and re-add to scene tree.
    test_node.remove_child(obj.clone().upcast());
    test_node.add_child(obj.clone().upcast());

    // _ready does NOT run again, implementation_value should still be 1.
    assert_eq!(obj.bind().implementation_value, 1);

    // Request ready.
    obj.clone().upcast::<Node>().request_ready();

    // Remove and re-add to scene tree.
    test_node.remove_child(obj.clone().upcast());
    test_node.add_child(obj.clone().upcast());

    // _ready runs again since we asked it to; implementation_value should be 2.
    assert_eq!(obj.bind().implementation_value, 2);
}

#[itest]
fn test_tree_enters_exits(test_context: &TestContext) {
    let obj = VirtualTreeTest::new_alloc();
    assert_eq!(obj.bind().tree_enters, 0);
    assert_eq!(obj.bind().tree_exits, 0);
    let mut test_node = test_context.scene_tree.clone();

    // Add to scene tree.
    test_node.add_child(obj.clone().upcast());
    assert_eq!(obj.bind().tree_enters, 1);
    assert_eq!(obj.bind().tree_exits, 0);

    // Remove and re-add to scene tree.
    test_node.remove_child(obj.clone().upcast());
    assert_eq!(obj.bind().tree_enters, 1);
    assert_eq!(obj.bind().tree_exits, 1);
    test_node.add_child(obj.clone().upcast());
    assert_eq!(obj.bind().tree_enters, 2);
    assert_eq!(obj.bind().tree_exits, 1);
}

#[itest]
fn test_virtual_method_with_return() {
    let obj = VirtualReturnTest::new_gd();
    let arr = obj.clone().upcast::<PrimitiveMesh>().get_mesh_arrays();
    let arr_rust = obj.bind().create_mesh_array();
    assert_eq!(arr.len(), arr_rust.len());
    // can't just assert_eq because the values of some floats change slightly
    assert_eq_approx!(
        arr.at(0).to::<PackedVector3Array>()[0],
        arr_rust.at(0).to::<PackedVector3Array>()[0],
    );
    assert_eq_approx!(
        real::from_f32(arr.at(2).to::<PackedFloat32Array>()[3]),
        real::from_f32(arr_rust.at(2).to::<PackedFloat32Array>()[3]),
    );
    assert_eq_approx!(
        arr.at(3).to::<PackedColorArray>()[0],
        arr_rust.at(3).to::<PackedColorArray>()[0],
    );
    assert_eq_approx!(
        arr.at(4).to::<PackedVector2Array>()[0],
        arr_rust.at(4).to::<PackedVector2Array>()[0],
    );
    assert_eq!(
        arr.at(6).to::<PackedByteArray>(),
        arr_rust.at(6).to::<PackedByteArray>(),
    );
    assert_eq!(
        arr.at(10).to::<PackedInt32Array>(),
        arr_rust.at(10).to::<PackedInt32Array>()
    );
}

#[itest]
fn test_format_loader(_test_context: &TestContext) {
    let format_loader = FormatLoaderTest::new_gd();
    let mut loader = ResourceLoader::singleton();
    loader
        .add_resource_format_loader_ex(format_loader.clone().upcast())
        .at_front(true)
        .done();

    let extensions = loader.get_recognized_extensions_for_type(FormatLoaderTest::resource_type());
    let mut extensions_rust = format_loader.bind().get_recognized_extensions();
    extensions_rust.push("tres".into());
    assert_eq!(extensions, extensions_rust);
    let resource = loader
        .load_ex("path.extension".into())
        .cache_mode(CacheMode::IGNORE)
        .done()
        .unwrap();
    assert!(resource.try_cast::<BoxMesh>().is_ok());

    loader.remove_resource_format_loader(format_loader.upcast());
}

#[itest]
fn test_input_event(test_context: &TestContext) {
    let obj = VirtualInputTest::new_alloc();
    assert_eq!(obj.bind().event, None);
    let mut test_viewport = Window::new_alloc();

    test_context
        .scene_tree
        .clone()
        .add_child(test_viewport.clone().upcast());

    test_viewport.clone().add_child(obj.clone().upcast());

    let mut event = InputEventAction::new_gd();
    event.set_action("debug".into());
    event.set_pressed(true);

    // We're running in headless mode, so Input.parse_input_event does not work
    test_viewport.clone().push_input(event.clone().upcast());

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

    test_context
        .scene_tree
        .clone()
        .add_child(test_viewport.clone().upcast());

    for obj in objs.iter() {
        test_viewport.clone().add_child(obj.clone().upcast())
    }

    let mut event = InputEventAction::new_gd();
    event.set_action("debug".into());
    event.set_pressed(true);

    // We're running in headless mode, so Input.parse_input_event does not work
    test_viewport.clone().push_input(event.clone().upcast());

    for obj in objs.iter() {
        assert_eq!(obj.bind().event, Some(event.clone().upcast::<InputEvent>()));
    }

    test_viewport.queue_free();
}

#[itest]
fn test_notifications() {
    let obj = NotificationTest::new_alloc();
    let mut node = obj.clone().upcast::<Node>();
    node.notify(NodeNotification::UNPAUSED);
    node.notify(NodeNotification::EDITOR_POST_SAVE);
    node.notify(NodeNotification::READY);
    node.notify_reversed(NodeNotification::WM_SIZE_CHANGED);

    assert_eq!(
        obj.bind().sequence,
        vec![
            ReceivedEvent::Notification(NodeNotification::UNPAUSED),
            ReceivedEvent::Notification(NodeNotification::EDITOR_POST_SAVE),
            ReceivedEvent::Ready,
            ReceivedEvent::Notification(NodeNotification::READY),
            ReceivedEvent::Notification(NodeNotification::WM_SIZE_CHANGED),
        ]
    );
    obj.free();
}

#[itest]
fn test_get_called() {
    let obj = GetTest::new_gd();
    assert!(!obj.bind().get_called.get());
    assert!(obj.get("foo".into()).is_nil());
    assert!(obj.bind().get_called.get());

    let obj = GetTest::new_gd();
    assert!(!obj.bind().get_called.get());
    obj.get("always_get_hello".into());
    assert!(obj.bind().get_called.get());
}

#[itest]
fn test_get_returns_correct() {
    let mut obj = GetTest::new_gd();

    {
        let mut obj = obj.bind_mut();
        obj.always_get_hello = 100;
        obj.gettable = 200;
    }

    assert_eq!(obj.get("always_get_hello".into()), "hello".to_variant());
    assert_eq!(obj.get("gettable".into()), 200.to_variant());
}

#[itest]
fn test_set_called() {
    let mut obj = SetTest::new_gd();
    assert!(!obj.bind().set_called);
    obj.set("foo".into(), Variant::nil());
    assert!(obj.bind().set_called);

    let mut obj = SetTest::new_gd();
    assert!(!obj.bind().set_called);
    obj.set("settable".into(), 20.to_variant());
    assert!(obj.bind().set_called);
}

#[itest]
fn test_set_sets_correct() {
    let mut obj = SetTest::new_gd();

    assert_eq!(obj.bind().always_set_to_100, i64::default());
    assert_eq!(obj.bind().settable, i64::default());
    obj.set("always_set_to_100".into(), "hello".to_variant());
    obj.set("settable".into(), 500.to_variant());
    assert_eq!(obj.bind().always_set_to_100, 100);
    assert_eq!(obj.bind().settable, 500);
}

#[itest]
fn test_revert() {
    let revert = RevertTest::new_gd();

    let not_revert = StringName::from("property_not_revert");
    let do_revert = StringName::from("property_do_revert");
    let changes = StringName::from("property_changes");

    assert!(!revert.property_can_revert(not_revert.clone()));
    assert_eq!(revert.property_get_revert(not_revert), Variant::nil());
    assert!(revert.property_can_revert(do_revert.clone()));
    assert_eq!(
        revert.property_get_revert(do_revert),
        GString::from("hello!").to_variant()
    );

    assert!(!revert.property_can_revert(changes.clone()));
    assert!(revert.property_can_revert(changes.clone()));

    assert_eq!(revert.property_get_revert(changes.clone()), Variant::nil());
    assert_eq!(
        revert.property_get_revert(changes.clone()),
        true.to_variant()
    );
}

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

#[derive(GodotClass)]
#[class(init)]
pub struct GetSetTest {
    #[var]
    always_get_100: i64,
    #[var]
    set_get: i64,

    get_called: std::cell::Cell<bool>,
    set_called: bool,
}

#[godot_api]
impl IRefCounted for GetSetTest {
    fn get_property(&self, property: StringName) -> Option<Variant> {
        self.get_called.set(true);

        match String::from(property).as_str() {
            "always_get_100" => Some(100.to_variant()),
            "set_get" => Some(self.set_get.to_variant()),
            _ => None,
        }
    }

    fn set_property(&mut self, property: StringName, value: Variant) -> bool {
        self.set_called = true;

        match String::from(property).as_str() {
            "always_get_100" => {
                self.always_get_100 = value.to();
                true
            }
            "set_get" => {
                self.set_get = value.to();
                true
            }
            _ => false,
        }
    }
}

#[godot_api]
impl GetSetTest {
    #[func]
    fn is_get_called(&self) -> bool {
        self.get_called.get()
    }

    #[func]
    fn unset_get_called(&mut self) {
        self.get_called.set(false)
    }

    #[func]
    fn is_set_called(&self) -> bool {
        self.set_called
    }

    #[func]
    fn unset_set_called(&mut self) {
        self.set_called = false
    }

    #[func]
    fn get_real_always_get_100(&self) -> i64 {
        self.always_get_100
    }
}
