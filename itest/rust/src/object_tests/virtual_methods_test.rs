/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::builtin::{
    real, varray, vslice, AnyArray, Color, GString, PackedByteArray, PackedColorArray,
    PackedFloat32Array, PackedInt32Array, PackedVector2Array, PackedVector3Array, RealConv,
    StringName, Variant, Vector2, Vector3,
};
use godot::classes::notify::NodeNotification;
#[cfg(feature = "codegen-full")]
use godot::classes::Material;
use godot::classes::{
    IEditorPlugin, INode, INode2D, IPrimitiveMesh, IRefCounted, InputEvent, InputEventAction, Node,
    Node2D, Object, PrimitiveMesh, RefCounted, Window,
};
use godot::global::godot_str;
use godot::meta::ToGodot;
use godot::obj::{Base, Gd, NewAlloc, NewGd};
use godot::private::class_macros::assert_eq_approx;
use godot::register::{godot_api, GodotClass};

use crate::framework::{itest, TestContext};

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
        godot_str!("VirtualMethodTest[integer={}]", self.integer)
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

#[rustfmt::skip]
#[godot_api]
impl IPrimitiveMesh for VirtualReturnTest {
    fn create_mesh_array(&self) -> AnyArray {
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
        ].upcast_any_array()
    }

    fn get_surface_count(&self) -> i32 { unreachable!() }
    fn surface_get_array_len(&self, _index: i32) -> i32 { unreachable!() }
    fn surface_get_array_index_len(&self, _index: i32) -> i32 { unreachable!() }
    fn surface_get_arrays(&self, _index: i32) -> AnyArray { unreachable!() }
    fn surface_get_blend_shape_arrays(&self, _index: i32) -> godot::prelude::Array<AnyArray> { unreachable!() }
    fn surface_get_lods(&self, _index: i32) -> godot::prelude::VarDictionary { unreachable!() }
    fn surface_get_format(&self, _index: i32) -> u32 { unreachable!() }
    fn surface_get_primitive_type(&self, _index: i32) -> u32 { unreachable!() }
    #[cfg(feature = "codegen-full")]
    fn surface_set_material(&mut self, _index: i32, _material: Option<Gd<Material>>) { unreachable!() }
    #[cfg(feature = "codegen-full")]
    fn surface_get_material(&self, _index: i32) -> Option<Gd<Material>> { unreachable!() }
    fn get_blend_shape_count(&self) -> i32 { unreachable!() }
    fn get_blend_shape_name(&self, _index: i32) -> StringName { unreachable!() }
    fn set_blend_shape_name(&mut self, _index: i32, _name: StringName) { unreachable!() }
    fn get_aabb(&self) -> godot::prelude::Aabb { unreachable!() }
}

#[derive(GodotClass, Debug)]
#[class(base=Node2D)]
pub(super) struct VirtualInputTest {
    base: Base<Node2D>,
    pub event: Option<Gd<InputEvent>>,
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
                if INC
                    .fetch_add(1, std::sync::atomic::Ordering::AcqRel)
                    .is_multiple_of(2)
                {
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

#[derive(GodotClass)]
#[class(init)]
struct VirtualGdSelfTest {
    #[init(val = 4)]
    some_val: i64,
}

#[godot_api]
impl IRefCounted for VirtualGdSelfTest {
    #[func(gd_self)]
    fn to_string(_this: Gd<Self>) -> GString {
        GString::from("Gd<Self>")
    }

    #[func(gd_self)]
    fn get_property(this: Gd<Self>, _property: StringName) -> Option<Variant> {
        // Delegates call to Display which calls `VirtualGdSelfTest::to_string` later in the chain.
        Some(this.to_string().to_variant())
    }

    #[func(gd_self)]
    fn set_property(mut this: Gd<Self>, _property: StringName, value: Variant) -> bool {
        // Check bind_mut and bind.
        this.bind_mut().some_val = value.to();
        this.bind().some_val != 4
    }

    #[func(gd_self)]
    fn property_get_revert(this: Gd<Self>, property: StringName) -> Option<Variant> {
        // Access other virtual method directly.
        let property = Self::get_property(this, property)?;
        Some(property)
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
    test_node.add_child(&obj);

    // _ready runs, increments implementation_value once.
    assert_eq!(obj.bind().implementation_value, 1);
}

#[itest]
fn test_ready_panic(test_context: &TestContext) {
    let mut obj = VirtualReadyTest::new_alloc();
    obj.bind_mut().panics = true;

    // Add to scene tree -- this panics.
    // NOTE: Current implementation catches panics, but does not propagate them to the user.
    // Godot has no mechanism to transport errors across ptrcalls (e.g. virtual function calls), so this would need to be emulated somehow.
    let mut test_node = test_context.scene_tree.clone();
    // expect_panic("panic in ready() propagated to caller", || {
    test_node.add_child(&obj);
    // });

    assert_eq!(obj.bind().implementation_value, 0);
}

#[itest]
fn test_ready_dynamic_panic(test_context: &TestContext) {
    let mut obj = VirtualReadyTest::new_alloc();
    obj.bind_mut().panics = true;

    // Add to scene tree -- this panics.
    let mut test_node = test_context.scene_tree.clone();

    // NOTE: Current implementation catches panics, but does not propagate them to the user.
    // Godot has no mechanism to transport errors across ptrcalls (e.g. virtual function calls), so this would need to be emulated somehow.
    let result = test_node.try_call("add_child", vslice![obj]);
    // let err = result.expect_err("add_child() should have panicked");
    let returned = result.expect("at the moment, panics in virtual functions are swallowed");
    assert_eq!(returned, Variant::nil());

    assert_eq!(obj.bind().implementation_value, 0);
}

#[itest]
fn test_ready_multiple_fires(test_context: &TestContext) {
    let obj = VirtualReadyTest::new_alloc();
    assert_eq!(obj.bind().implementation_value, 0);

    let mut test_node = test_context.scene_tree.clone();

    // Add to scene tree.
    test_node.add_child(&obj);

    // _ready runs, increments implementation_value once.
    assert_eq!(obj.bind().implementation_value, 1);

    // Remove and re-add to scene tree.
    test_node.remove_child(&obj);
    test_node.add_child(&obj);

    // _ready does NOT run again, implementation_value should still be 1.
    assert_eq!(obj.bind().implementation_value, 1);
}

#[itest]
fn test_ready_request_ready(test_context: &TestContext) {
    let obj = VirtualReadyTest::new_alloc();
    assert_eq!(obj.bind().implementation_value, 0);

    let mut test_node = test_context.scene_tree.clone();

    // Add to scene tree.
    test_node.add_child(&obj);

    // _ready runs, increments implementation_value once.
    assert_eq!(obj.bind().implementation_value, 1);

    // Remove and re-add to scene tree.
    test_node.remove_child(&obj);
    test_node.add_child(&obj);

    // _ready does NOT run again, implementation_value should still be 1.
    assert_eq!(obj.bind().implementation_value, 1);

    // Request ready.
    obj.clone().upcast::<Node>().request_ready();

    // Remove and re-add to scene tree.
    test_node.remove_child(&obj);
    test_node.add_child(&obj);

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
    test_node.add_child(&obj);
    assert_eq!(obj.bind().tree_enters, 1);
    assert_eq!(obj.bind().tree_exits, 0);

    // Remove and re-add to scene tree.
    test_node.remove_child(&obj);
    assert_eq!(obj.bind().tree_enters, 1);
    assert_eq!(obj.bind().tree_exits, 1);
    test_node.add_child(&obj);
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
            #[cfg(since_api = "4.4")]
            ReceivedEvent::Notification(NodeNotification::POSTINITIALIZE),
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
    assert!(obj.get("inexistent").is_nil());
    assert!(obj.bind().get_called.get());

    let obj = GetTest::new_gd();
    assert!(!obj.bind().get_called.get());
    obj.get("always_get_hello");
    assert!(obj.bind().get_called.get());
}

#[itest]
fn test_get_returns() {
    let mut obj = GetTest::new_gd();

    {
        let mut obj = obj.bind_mut();
        obj.always_get_hello = 100;
        obj.gettable = 200;
    }

    assert_eq!(obj.get("always_get_hello"), "hello".to_variant());
    assert_eq!(obj.get("gettable"), 200.to_variant());
}

#[itest]
fn test_set_called() {
    let mut obj = SetTest::new_gd();
    assert!(!obj.bind().set_called);
    obj.set("inexistent_property", &Variant::nil());
    assert!(obj.bind().set_called);

    let mut obj = SetTest::new_gd();
    assert!(!obj.bind().set_called);
    obj.set("settable", &20.to_variant());
    assert!(obj.bind().set_called);
}

#[itest]
fn test_set_sets() {
    let mut obj = SetTest::new_gd();

    assert_eq!(obj.bind().always_set_to_100, i64::default());
    assert_eq!(obj.bind().settable, i64::default());
    obj.set("always_set_to_100", &"hello".to_variant());
    obj.set("settable", &500.to_variant());
    assert_eq!(obj.bind().always_set_to_100, 100);
    assert_eq!(obj.bind().settable, 500);
}

#[itest]
fn test_revert() {
    let revert = RevertTest::new_gd();

    let not_revert = StringName::from("property_not_revert");
    let do_revert = StringName::from("property_do_revert");
    let changes = StringName::from("property_changes");

    assert!(!revert.property_can_revert(&not_revert));
    assert_eq!(revert.property_get_revert(&not_revert), Variant::nil());
    assert!(revert.property_can_revert(&do_revert));
    assert_eq!(
        revert.property_get_revert(&do_revert),
        "hello!".to_variant()
    );

    assert!(!revert.property_can_revert(&changes));
    assert!(revert.property_can_revert(&changes));

    assert_eq!(revert.property_get_revert(&changes), Variant::nil());
    assert_eq!(revert.property_get_revert(&changes), true.to_variant());
}

#[itest]
fn test_gd_self_virtual_methods() {
    let mut obj = VirtualGdSelfTest::new_gd();
    let expected = GString::from("Gd<Self>");

    // Test various calling conventions:
    let ret: GString = obj.call("to_string", &[]).to::<GString>();
    assert_eq!(ret, expected);

    let ret: GString = obj.call("get", vslice![""]).to();
    assert_eq!(ret, expected);

    obj.set("a", &4.to_variant());
    assert_eq!(obj.bind().some_val, 4);

    let ret: GString = obj.property_get_revert("a").to();
    assert_eq!(ret, expected);
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

// ----------------------------------------------------------------------------------------------------------------------------------------------

// There isn't a good way to test editor plugins, but we can at least declare one to ensure that the macro compiles.
#[derive(GodotClass)]
#[class(init, base = EditorPlugin, tool)]
struct CustomEditorPlugin;

// Just override EditorPlugin::edit() to verify method is declared with Option<T>.
// See https://github.com/godot-rust/gdext/issues/494.
#[godot_api]
impl IEditorPlugin for CustomEditorPlugin {
    fn edit(&mut self, _object: Option<Gd<Object>>) {
        // Do nothing.
    }

    // This parameter is non-null.
    fn handles(&self, _object: Gd<Object>) -> bool {
        true
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Test that virtual methods with u64 parameters work correctly.
///
/// `u64` doesn't have ToGodot/FromGodot implementations (not natively supported in GDScript),
/// but engine virtual methods may use it via EngineToGodot/EngineFromGodot.
#[cfg(feature = "codegen-full")]
#[derive(GodotClass)]
#[class(init, tool, base=OpenXrExtensionWrapper)]
struct VirtualU64Test {
    base: Base<godot::classes::OpenXrExtensionWrapper>,
}

#[cfg(feature = "codegen-full")]
#[godot_api]
impl godot::classes::IOpenXrExtensionWrapper for VirtualU64Test {
    fn on_instance_created(&mut self, _instance: u64) {
        // No need to do anything, this must just compile with u64.
    }
}
