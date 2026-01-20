/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::classes::node::DuplicateFlags;
use godot::classes::resource::DeepDuplicateMode;
use godot::classes::{Node, Node2D, Resource};
use godot::prelude::*;

use crate::framework::{expect_panic, itest};

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Node duplication tests

#[itest]
fn duplicate_node() {
    let mut node2d = Node2D::new_alloc();
    node2d.set_position(Vector2::new(10.0, 20.0));

    let dup = node2d.duplicate_node();
    assert_eq!(dup.get_position(), Vector2::new(10.0, 20.0));

    node2d.free();
    dup.free();
}

#[itest]
fn duplicate_node_ex() {
    let mut node = Node::new_alloc();
    node.set_name("test_node");

    // Test flags() method with combined flags.
    let dup1 = node
        .duplicate_node_ex()
        .flags(DuplicateFlags::SIGNALS | DuplicateFlags::GROUPS | DuplicateFlags::SCRIPTS)
        .done();

    assert_eq!(dup1.get_name(), "test_node");

    // Test flags() method with different flags.
    let dup2 = node
        .duplicate_node_ex()
        .flags(DuplicateFlags::SIGNALS | DuplicateFlags::GROUPS)
        .done();

    assert_eq!(dup2.get_name(), "test_node");

    node.free();
    dup1.free();
    dup2.free();
}

#[itest]
fn duplicate_node_groups_behavior(ctx: &crate::framework::TestContext) {
    let mut node = Node::new_alloc();
    node.set_name("grouped_node");
    ctx.scene_tree.clone().add_child(&node);

    node.add_to_group("test_group");
    assert!(node.is_in_group("test_group"));

    // Duplicate with groups flag should preserve group membership.
    let dup_with_groups = node
        .duplicate_node_ex()
        .flags(DuplicateFlags::GROUPS)
        .done();
    ctx.scene_tree.clone().add_child(&dup_with_groups);
    assert!(dup_with_groups.is_in_group("test_group"));

    // Duplicate without groups flag should not preserve group membership.
    let dup_without_groups = node
        .duplicate_node_ex()
        .flags(DuplicateFlags::from_ord(0))
        .done();
    ctx.scene_tree.clone().add_child(&dup_without_groups);
    assert!(!dup_without_groups.is_in_group("test_group"));

    node.free();
    dup_with_groups.free();
    dup_without_groups.free();
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Resource duplication tests

#[itest]
fn duplicate_resource_simple() {
    let resource = Resource::new_gd();

    let dup = resource.duplicate_resource();

    assert_ne!(dup, resource);
}

#[itest]
fn duplicate_resource_ex_modes() {
    let resource = Resource::new_gd();

    // Godot's older duplicate(deep=true) API.
    let dup_deep = resource.duplicate_resource_ex().deep();

    // Newer duplicate_deep(mode=INTERNAL) API.
    let dup_internal = resource
        .duplicate_resource_ex()
        .deep_subresources(DeepDuplicateMode::INTERNAL);
    let dup_all = resource
        .duplicate_resource_ex()
        .deep_subresources(DeepDuplicateMode::ALL);

    assert_ne!(dup_deep, resource);
    assert_ne!(dup_internal, resource);
    assert_ne!(dup_all, resource);
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Polymorphic duplication failure tests

#[derive(GodotClass)]
#[class(no_init, base = Node)]
struct NoInitNode {}

#[itest]
fn duplicate_node_no_init_fails() {
    // Create a NoInitNode by manually constructing the struct.
    // This simulates having a node instance that cannot be default-constructed.
    let no_init_node = Gd::from_object(NoInitNode {});

    // When we have a Gd<Node> that points to a NoInitNode at runtime,
    // duplication should panic because NoInitNode is not default-constructible.
    let node_ptr: Gd<Node> = no_init_node.upcast();

    expect_panic("no_init node duplication", || {
        let _dup = node_ptr.duplicate_node();
    });

    node_ptr.free();
}

#[derive(GodotClass)]
#[class(no_init, base = Resource)]
struct NoInitResource {}

#[itest]
fn duplicate_resource_no_init_fails() {
    // Create a NoInitResource by manually constructing the struct.
    // This simulates having a resource instance that cannot be default-constructed.
    let no_init_resource = Gd::from_object(NoInitResource {});

    // When we have a Gd<Resource> that points to a NoInitResource at runtime,
    // duplication should panic because NoInitResource is not default-constructible.
    let resource_ptr: Gd<Resource> = no_init_resource.upcast();

    expect_panic("no_init resource duplication", || {
        let _dup = resource_ptr.duplicate_resource();
    });
}
