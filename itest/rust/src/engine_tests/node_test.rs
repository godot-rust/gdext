/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::str::FromStr;

use godot::builtin::{vslice, NodePath};
use godot::classes::{Node, Node3D, PackedScene, SceneTree};
use godot::global;
use godot::obj::{NewAlloc, NewGd};

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

#[itest(skip)]
fn node_scene_tree() {
    let mut child = Node::new_alloc();
    child.set_name("kid");

    let mut parent = Node::new_alloc();
    parent.set_name("parent");
    parent.add_child(&child);

    let mut scene = PackedScene::new_gd();
    let err = scene.pack(&parent);
    assert_eq!(err, global::Error::OK);

    let mut tree = SceneTree::new_alloc();
    let err = tree.change_scene_to_packed(&scene);
    assert_eq!(err, global::Error::OK);

    // Note: parent + child are not owned by PackedScene, thus need to be freed
    // (verified by porting this very test to GDScript)
    tree.free();
    parent.free();
    child.free();
}

#[itest]
fn node_call_group(ctx: &TestContext) {
    let mut node = ctx.scene_tree.clone();
    let mut tree = node.get_tree().unwrap();

    node.add_to_group("group");
    tree.call_group("group", "set_name", vslice!["name"]);
}
