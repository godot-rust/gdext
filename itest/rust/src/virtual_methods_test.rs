/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

#![allow(dead_code)]

use crate::TestContext;
use godot::bind::{godot_api, GodotClass};
use godot::builtin::GodotString;
use godot::engine::node::InternalMode;
use godot::engine::{Node, Node2D, Node2DVirtual, NodeVirtual, RefCounted, RefCountedVirtual};
use godot::obj::{Base, Gd, Share};
use godot::prelude::PackedStringArray;
use godot::test::itest;

/// Simple class, that deliberately has no constructor accessible from GDScript
#[derive(GodotClass, Debug)]
#[class(base=RefCounted)]
struct WithoutInit {
    #[base]
    some_base: Base<RefCounted>,
}

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
impl RefCountedVirtual for VirtualMethodTest {
    fn to_string(&self) -> GodotString {
        format!("VirtualMethodTest[integer={}]", self.integer).into()
    }
}

#[derive(GodotClass, Debug)]
#[class(base=Node2D)]
struct ReadyVirtualTest {
    #[base]
    some_base: Base<Node2D>,
    implementation_value: i32,
}

#[godot_api]
impl Node2DVirtual for ReadyVirtualTest {
    fn init(base: Base<Node2D>) -> Self {
        ReadyVirtualTest {
            some_base: base,
            implementation_value: 0,
        }
    }

    fn ready(&mut self) {
        self.implementation_value += 1;
    }
}

#[derive(GodotClass, Debug)]
#[class(base=Node2D)]
struct TreeVirtualTest {
    #[base]
    some_base: Base<Node2D>,
    tree_enters: i32,
    tree_exits: i32,
}

#[godot_api]
impl Node2DVirtual for TreeVirtualTest {
    fn init(base: Base<Node2D>) -> Self {
        TreeVirtualTest {
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

#[derive(GodotClass, Debug)]
#[class(base=Node)]
struct ReturnVirtualTest {
    #[base]
    base: Base<Node>,
}

#[godot_api]
impl NodeVirtual for ReturnVirtualTest {
    fn init(base: Base<Node>) -> Self {
        ReturnVirtualTest { base }
    }

    fn get_configuration_warnings(&self) -> PackedStringArray {
        let mut output = PackedStringArray::new();
        output.push("Hello".into());
        output
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[itest]
fn test_to_string() {
    let _obj = Gd::<VirtualMethodTest>::new_default();
}

#[itest]
fn test_ready(test_context: &TestContext) {
    let obj = Gd::<ReadyVirtualTest>::new_default();
    assert_eq!(obj.bind().implementation_value, 0);

    // Add to scene tree
    let mut test_node = test_context.scene_tree.share();
    test_node.add_child(
        obj.share().upcast(),
        false,
        InternalMode::INTERNAL_MODE_DISABLED,
    );

    // _ready runs, increments implementation_value once.
    assert_eq!(obj.bind().implementation_value, 1);
}

#[itest]
fn test_ready_multiple_fires(test_context: &TestContext) {
    let obj = Gd::<ReadyVirtualTest>::new_default();
    assert_eq!(obj.bind().implementation_value, 0);

    let mut test_node = test_context.scene_tree.share();

    // Add to scene tree
    test_node.add_child(
        obj.share().upcast(),
        false,
        InternalMode::INTERNAL_MODE_DISABLED,
    );

    // _ready runs, increments implementation_value once.
    assert_eq!(obj.bind().implementation_value, 1);

    // Remove and re-add to scene tree
    test_node.remove_child(obj.share().upcast());
    test_node.add_child(
        obj.share().upcast(),
        false,
        InternalMode::INTERNAL_MODE_DISABLED,
    );

    // _ready does NOT run again, implementation_value should still be 1.
    assert_eq!(obj.bind().implementation_value, 1);
}

#[itest]
fn test_ready_request_ready(test_context: &TestContext) {
    let obj = Gd::<ReadyVirtualTest>::new_default();
    assert_eq!(obj.bind().implementation_value, 0);

    let mut test_node = test_context.scene_tree.share();

    // Add to scene tree
    test_node.add_child(
        obj.share().upcast(),
        false,
        InternalMode::INTERNAL_MODE_DISABLED,
    );

    // _ready runs, increments implementation_value once.
    assert_eq!(obj.bind().implementation_value, 1);

    // Remove and re-add to scene tree
    test_node.remove_child(obj.share().upcast());
    test_node.add_child(
        obj.share().upcast(),
        false,
        InternalMode::INTERNAL_MODE_DISABLED,
    );

    // _ready does NOT run again, implementation_value should still be 1.
    assert_eq!(obj.bind().implementation_value, 1);

    // Request ready
    obj.share().upcast::<Node>().request_ready();

    // Remove and re-add to scene tree
    test_node.remove_child(obj.share().upcast());
    test_node.add_child(
        obj.share().upcast(),
        false,
        InternalMode::INTERNAL_MODE_DISABLED,
    );

    // _ready runs again since we asked it to; implementation_value should be 2.
    assert_eq!(obj.bind().implementation_value, 2);
}

#[itest]
fn test_tree_enters_exits(test_context: &TestContext) {
    let obj = Gd::<TreeVirtualTest>::new_default();
    assert_eq!(obj.bind().tree_enters, 0);
    assert_eq!(obj.bind().tree_exits, 0);
    let mut test_node = test_context.scene_tree.share();

    // Add to scene tree
    test_node.add_child(
        obj.share().upcast(),
        false,
        InternalMode::INTERNAL_MODE_DISABLED,
    );
    assert_eq!(obj.bind().tree_enters, 1);
    assert_eq!(obj.bind().tree_exits, 0);

    // Remove and re-add to scene tree
    test_node.remove_child(obj.share().upcast());
    assert_eq!(obj.bind().tree_enters, 1);
    assert_eq!(obj.bind().tree_exits, 1);
    test_node.add_child(
        obj.share().upcast(),
        false,
        InternalMode::INTERNAL_MODE_DISABLED,
    );
    assert_eq!(obj.bind().tree_enters, 2);
    assert_eq!(obj.bind().tree_exits, 1);
}

#[itest]
fn test_virtual_method_with_return(_test_context: &TestContext) {
    let obj = Gd::<ReturnVirtualTest>::new_default();
    let output = obj.bind().get_configuration_warnings();
    assert!(output.contains("Hello".into()));
    assert_eq!(output.len(), 1);
    obj.free();
}
