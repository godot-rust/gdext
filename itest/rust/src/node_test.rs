/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::{itest, TestContext};
use godot::builtin::{NodePath, Variant};
use godot::engine::{global, node, Node, Node3D, NodeExt, PackedScene, SceneTree};
use godot::obj::Share;

#[itest]
fn node_get_node() {
    let mut child = Node3D::new_alloc();
    child.set_name("child".into());
    let child_id = child.instance_id();

    let mut parent = Node3D::new_alloc();
    parent.set_name("parent".into());
    parent.add_child(
        child.share().upcast(),
        false,
        node::InternalMode::INTERNAL_MODE_DISABLED,
    );

    let mut grandparent = Node::new_alloc();
    grandparent.set_name("grandparent".into());
    grandparent.add_child(
        parent.share().upcast(),
        false,
        node::InternalMode::INTERNAL_MODE_DISABLED,
    );

    // Directly on Gd<T>
    let found = grandparent.get_node_as::<Node3D>(NodePath::from("parent/child"));
    assert_eq!(found.instance_id(), child_id);

    // Deref via &T
    let found = grandparent.try_get_node_as::<Node3D>(NodePath::from("parent/child"));
    let found = found.expect("try_get_node_as() returned Some(..)");
    assert_eq!(found.instance_id(), child_id);

    grandparent.free();
}

#[itest]
fn node_get_node_fail() {
    let mut child = Node3D::new_alloc();
    child.set_name("child".into());

    let found = child.try_get_node_as::<Node3D>(NodePath::from("non-existent"));
    assert!(found.is_none());

    child.free();
}

#[itest(skip)]
fn node_scene_tree() {
    let mut child = Node::new_alloc();
    child.set_name("kid".into());

    let mut parent = Node::new_alloc();
    parent.set_name("parent".into());
    parent.add_child(
        child.share(),
        false,
        node::InternalMode::INTERNAL_MODE_DISABLED,
    );

    let mut scene = PackedScene::new();
    let err = scene.pack(parent.share());
    assert_eq!(err, global::Error::OK);

    let mut tree = SceneTree::new_alloc();
    let err = tree.change_scene_to_packed(scene);
    assert_eq!(err, global::Error::OK);

    // Note: parent + child are not owned by PackedScene, thus need to be freed
    // (verified by porting this very test to GDScript)
    tree.free();
    parent.free();
    child.free();
}

// FIXME: call_group() crashes
#[itest(skip)]
fn node_call_group(ctx: &TestContext) {
    let mut node = ctx.scene_tree.share();
    let mut tree = node.get_tree().unwrap();

    node.add_to_group("group".into(), true);
    tree.call_group("group".into(), "set_name".into(), &[Variant::from("name")]);
}
