/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

#![allow(dead_code)]

use crate::framework::{itest, TestContext};

use godot::bind::{godot_api, GodotClass};
use godot::builtin::meta::ToGodot;
use godot::builtin::{
    real, varray, Color, GString, PackedByteArray, PackedColorArray, PackedFloat32Array,
    PackedInt32Array, PackedStringArray, PackedVector2Array, PackedVector3Array, RealConv,
    StringName, Variant, VariantArray, Vector2, Vector3,
};
use godot::engine::notify::NodeNotification;
use godot::engine::resource_loader::CacheMode;
use godot::engine::{
    BoxMesh, INode, INode2D, IPrimitiveMesh, IRefCounted, IResourceFormatLoader, IRigidBody2D,
    InputEvent, InputEventAction, Node, Node2D, PrimitiveMesh, RefCounted, ResourceFormatLoader,
    ResourceLoader, Viewport, Window,
};
use godot::obj::{Base, Gd, UserClass};
use godot::private::class_macros::assert_eq_approx;

/// Simple class, that deliberately has no constructor accessible from GDScript
#[derive(GodotClass, Debug)]
#[class(base=RefCounted)]
struct WithoutInit {
    #[base]
    some_base: Base<RefCounted>,
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[derive(GodotClass, Debug)]
#[class(init, base=RefCounted)]
struct VirtualMethodTest {
    #[base]
    some_base: Base<RefCounted>,

    integer: i32,
}

#[godot_api]
impl VirtualMethodTest {}

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
    #[base]
    some_base: Base<Node2D>,
    implementation_value: i32,
}

#[godot_api]
impl INode2D for VirtualReadyTest {
    fn init(base: Base<Node2D>) -> Self {
        VirtualReadyTest {
            some_base: base,
            implementation_value: 0,
        }
    }

    fn ready(&mut self) {
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
    #[base]
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
    #[base]
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
    #[base]
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
    #[base]
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
        BoxMesh::new().to_variant()
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
    #[base]
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

#[itest]
fn test_to_string() {
    let _obj = VirtualMethodTest::new_gd();
}

#[itest]
fn test_ready(test_context: &TestContext) {
    let obj = VirtualReadyTest::alloc_gd();
    assert_eq!(obj.bind().implementation_value, 0);

    // Add to scene tree
    let mut test_node = test_context.scene_tree.clone();
    test_node.add_child(obj.clone().upcast());

    // _ready runs, increments implementation_value once.
    assert_eq!(obj.bind().implementation_value, 1);
}

#[itest]
fn test_ready_multiple_fires(test_context: &TestContext) {
    let obj = VirtualReadyTest::alloc_gd();
    assert_eq!(obj.bind().implementation_value, 0);

    let mut test_node = test_context.scene_tree.clone();

    // Add to scene tree
    test_node.add_child(obj.clone().upcast());

    // _ready runs, increments implementation_value once.
    assert_eq!(obj.bind().implementation_value, 1);

    // Remove and re-add to scene tree
    test_node.remove_child(obj.clone().upcast());
    test_node.add_child(obj.clone().upcast());

    // _ready does NOT run again, implementation_value should still be 1.
    assert_eq!(obj.bind().implementation_value, 1);
}

#[itest]
fn test_ready_request_ready(test_context: &TestContext) {
    let obj = VirtualReadyTest::alloc_gd();
    assert_eq!(obj.bind().implementation_value, 0);

    let mut test_node = test_context.scene_tree.clone();

    // Add to scene tree
    test_node.add_child(obj.clone().upcast());

    // _ready runs, increments implementation_value once.
    assert_eq!(obj.bind().implementation_value, 1);

    // Remove and re-add to scene tree
    test_node.remove_child(obj.clone().upcast());
    test_node.add_child(obj.clone().upcast());

    // _ready does NOT run again, implementation_value should still be 1.
    assert_eq!(obj.bind().implementation_value, 1);

    // Request ready
    obj.clone().upcast::<Node>().request_ready();

    // Remove and re-add to scene tree
    test_node.remove_child(obj.clone().upcast());
    test_node.add_child(obj.clone().upcast());

    // _ready runs again since we asked it to; implementation_value should be 2.
    assert_eq!(obj.bind().implementation_value, 2);
}

#[itest]
fn test_tree_enters_exits(test_context: &TestContext) {
    let obj = VirtualTreeTest::alloc_gd();
    assert_eq!(obj.bind().tree_enters, 0);
    assert_eq!(obj.bind().tree_exits, 0);
    let mut test_node = test_context.scene_tree.clone();

    // Add to scene tree
    test_node.add_child(obj.clone().upcast());
    assert_eq!(obj.bind().tree_enters, 1);
    assert_eq!(obj.bind().tree_exits, 0);

    // Remove and re-add to scene tree
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
        arr.get(0).to::<PackedVector3Array>().get(0),
        arr_rust.get(0).to::<PackedVector3Array>().get(0),
    );
    assert_eq_approx!(
        real::from_f32(arr.get(2).to::<PackedFloat32Array>().get(3)),
        real::from_f32(arr_rust.get(2).to::<PackedFloat32Array>().get(3)),
    );
    assert_eq_approx!(
        arr.get(3).to::<PackedColorArray>().get(0),
        arr_rust.get(3).to::<PackedColorArray>().get(0),
    );
    assert_eq_approx!(
        arr.get(4).to::<PackedVector2Array>().get(0),
        arr_rust.get(4).to::<PackedVector2Array>().get(0),
    );
    assert_eq!(
        arr.get(6).to::<PackedByteArray>(),
        arr_rust.get(6).to::<PackedByteArray>(),
    );
    assert_eq!(
        arr.get(10).to::<PackedInt32Array>(),
        arr_rust.get(10).to::<PackedInt32Array>()
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
        .cache_mode(CacheMode::CACHE_MODE_IGNORE)
        .done()
        .unwrap();
    assert!(resource.try_cast::<BoxMesh>().is_some());

    loader.remove_resource_format_loader(format_loader.upcast());
}

#[itest]
fn test_input_event(test_context: &TestContext) {
    let obj = VirtualInputTest::alloc_gd();
    assert_eq!(obj.bind().event, None);
    let mut test_viewport = Window::new_alloc();

    test_context
        .scene_tree
        .clone()
        .add_child(test_viewport.clone().upcast());

    test_viewport.clone().add_child(obj.clone().upcast());

    let mut event = InputEventAction::new();
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
        let obj = VirtualInputTest::alloc_gd();
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

    let mut event = InputEventAction::new();
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
    let obj = NotificationTest::alloc_gd();
    let mut node = obj.clone().upcast::<Node>();
    node.notify(NodeNotification::Unpaused);
    node.notify(NodeNotification::EditorPostSave);
    node.notify(NodeNotification::Ready);
    node.notify_reversed(NodeNotification::WmSizeChanged);

    assert_eq!(
        obj.bind().sequence,
        vec![
            ReceivedEvent::Notification(NodeNotification::Unpaused),
            ReceivedEvent::Notification(NodeNotification::EditorPostSave),
            ReceivedEvent::Ready,
            ReceivedEvent::Notification(NodeNotification::Ready),
            ReceivedEvent::Notification(NodeNotification::WmSizeChanged),
        ]
    );
    obj.free();
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
