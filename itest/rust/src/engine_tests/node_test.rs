/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::str::FromStr;

use godot::builtin::{vslice, NodePath};
use godot::classes::{Node, Node3D};
use godot::obj::NewAlloc;

use crate::framework::{itest, TestContext};

#[itest]
fn node_get_node() {
    let mut child = Node3D::new_alloc();
    child.set_name("child");
    let child_id = child.instance_id();

    let mut parent = Node3D::new_alloc();
    parent.set_name("parent");
    parent.add_child(&child);

    let mut grandparent = Node::new_alloc();
    grandparent.set_name("grandparent");
    grandparent.add_child(&parent);

    // Directly on Gd<T>
    let found = grandparent.get_node_as::<Node3D>("parent/child");
    assert_eq!(found.instance_id(), child_id);

    // Deref via &T
    let found = grandparent.try_get_node_as::<Node3D>(&NodePath::from("parent/child"));
    let found = found.expect("try_get_node_as() returned Some(..)");
    assert_eq!(found.instance_id(), child_id);

    grandparent.free();
}

#[itest]
fn node_get_node_fail() {
    let mut child = Node3D::new_alloc();
    child.set_name("child");

    let found = child.try_get_node_as::<Node3D>("non-existent");
    assert!(found.is_none());

    child.free();
}

#[itest]
fn node_path_from_str(ctx: &TestContext) {
    let child = ctx.scene_tree.clone();
    assert_eq!(
        child.get_path().to_string(),
        NodePath::from_str("/root/TestRunner").unwrap().to_string()
    );
}

// Regression test against call_group() crashing, see https://github.com/godot-rust/gdext/pull/167.
// https://github.com/godot-rust/gdext/commit/207c4e72ac0c24cfb83bab16f856dd09ebc8671c
#[itest]
fn node_call_group(ctx: &TestContext) {
    let mut node = ctx.scene_tree.clone();
    let mut tree = node.get_tree().unwrap();

    node.add_to_group("group");

    tree.call_group("group", "set_meta", vslice!["something", true]);
    assert!(node.has_meta("something"));

    tree.call_group("group", "remove_meta", vslice!["something"]);
    assert!(!node.has_meta("something"));
}

// Required parameter/return value.
#[cfg(all(feature = "codegen-full", since_api = "4.6"))]
#[itest]
fn node_required_param_return() {
    use godot::classes::Tween;
    use godot::obj::Gd;

    let mut parent = Node::new_alloc();
    let child = Node::new_alloc();

    // add_child() takes required arg, so this still works.
    // (Test for Option *not* working anymore is in godot > no_compile_tests.)
    parent.add_child(&child);

    // create_tween() returns now non-null instance.
    let tween: Gd<Tween> = parent.create_tween();
    assert!(tween.is_instance_valid());
    assert!(tween.to_string().contains("Tween"));

    parent.free();
}
