/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::classes::node::DuplicateFlags;
use godot::classes::{Node, Node2D, Resource};
use godot::global;
use godot::init::GdextBuild;
use godot::prelude::*;

use crate::framework::{expect_panic, itest};

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Node duplication tests

#[itest]
fn duplicate_node() {
    let mut node2d = Node2D::new_alloc();
    node2d.set_position(Vector2::new(10.0, 20.0));

    let copy = node2d.duplicate_node();
    assert_eq!(copy.get_position(), Vector2::new(10.0, 20.0));

    node2d.free();
    copy.free();
}

#[itest]
fn duplicate_node_ex() {
    let mut node = Node::new_alloc();
    node.add_to_group("grupi");

    let copy_default = node.duplicate_node_ex().done();
    let copy_grouped = node
        .duplicate_node_ex()
        .flags(DuplicateFlags::GROUPS)
        .done();
    let copy_ungrouped = node
        .duplicate_node_ex()
        .flags(DuplicateFlags::SIGNALS)
        .done();

    assert!(copy_default.is_in_group("grupi"));
    assert!(copy_grouped.is_in_group("grupi"));
    assert!(!copy_ungrouped.is_in_group("grupi"));

    node.free();
    copy_default.free();
    copy_grouped.free();
    copy_ungrouped.free();
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
// Resource duplication test infra

/// Test resource with `Array`, `Dictionary`, and direct `Gd<Resource>` property.
///
/// Terminology:
/// * **Internal subresource**: A resource that is not saved to a file (does not have a path).
///   When duplicating the parent, internal subresources are typically duplicated as well (deep copy).
/// * **External subresource**: A resource that is saved to a file (has a path, e.g. `res://...`).
///   When duplicating the parent, external subresources are typically shared (shallow copy), to preserve the reference to the file on disk.
#[derive(GodotClass)]
#[class(init, base = Resource)]
struct DupResource {
    #[export(storage)]
    array_with_resources: Array<Gd<Resource>>,

    #[export(storage)]
    dict_with_resources: VarDictionary,

    #[export(storage)]
    property_with_resource: Option<Gd<Resource>>,
}

/// Creates a test resource with internal/external subresources in Array, Dict, and as direct property.
fn create_test_resource() -> Gd<DupResource> {
    let internal = Resource::new_gd();
    let mut external = Resource::new_gd();
    external.set_path("res://fake_external.tres");
    let property = Resource::new_gd();

    let array: Array<Gd<Resource>> = array![&internal, &external];
    let dict: VarDictionary = vdict! {
        "internal": internal.clone(),
        "external": external.clone(),
    };

    let mut resource = DupResource::new_gd();
    resource.bind_mut().array_with_resources = array;
    resource.bind_mut().dict_with_resources = dict;
    resource.bind_mut().property_with_resource = Some(property.clone());

    resource
}

/// Verifies all duplication behaviors: subresource identity and container sharing.
///
/// Parameters specify which subresources/containers are expected to be duplicated.
/// * `internal_dup`: internal subresources.
/// * `external_dup`: external subresources (with path).
/// * `direct_dup`: direct subresource property.
/// * `container_dup`: container (array/dictionary) is duplicated -- shallow copy of contents, but new container identity.
fn verify_duplication(
    orig: &Gd<DupResource>,
    copy: &Gd<DupResource>,
    internal_dup: bool,
    external_dup: bool,
    direct_dup: bool,
    container_dup: bool,
) {
    let check = |sub: &Gd<Resource>, orig: &Gd<Resource>, expect_dup: bool, name: &str| {
        let is_dup = sub != orig;

        assert_eq!(
            is_dup, expect_dup,
            "Mismatch for {name}: expected dup={expect_dup}, got {is_dup}"
        );
    };

    let orig = orig.bind();
    let copy = copy.bind();

    check(
        &copy.array_with_resources.at(0),
        &orig.array_with_resources.at(0),
        internal_dup,
        "array_internal",
    );
    check(
        &copy.array_with_resources.at(1),
        &orig.array_with_resources.at(1),
        external_dup,
        "array_external",
    );
    check(
        &copy.dict_with_resources.at("internal").to(),
        &orig.dict_with_resources.at("internal").to(),
        internal_dup,
        "dict_internal",
    );
    check(
        &copy.dict_with_resources.at("external").to(),
        &orig.dict_with_resources.at("external").to(),
        external_dup,
        "dict_external",
    );
    check(
        &copy.property_with_resource.clone().unwrap(),
        &orig.property_with_resource.clone().unwrap(),
        direct_dup,
        "direct",
    );

    // Check Array/Dict identity -- must be same if `container_dup` is false.
    let same = global::is_same(
        &copy.dict_with_resources.to_variant(),
        &orig.dict_with_resources.to_variant(),
    );

    assert_ne!(same, container_dup, "Mismatch for container duplication");
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Resource tests

#[itest]
fn duplicate_resource_shallow() {
    // Tests both duplicate_resource() and duplicate_resource_ex().done() (same behavior).
    let orig = create_test_resource();
    let copy = orig.duplicate_resource();

    if GdextBuild::since_api("4.5") {
        // 4.5+: Everything just referenced (truly shallow).
        verify_duplication(&orig, &copy, false, false, false, false);
    } else {
        // 4.2-4.4: Everything just referenced (truly shallow), containers shallow-copied.
        verify_duplication(&orig, &copy, false, false, false, true);
    }
}

#[itest]
fn duplicate_resource_deep_internal() {
    let orig = create_test_resource();
    let copy = orig.duplicate_resource_ex().deep_internal().done();

    if GdextBuild::since_api("4.5") {
        // 4.5+: Internal subresources duplicated, external shared, array/dict deep-copied.
        verify_duplication(&orig, &copy, true, false, true, true);
    } else {
        // 4.2-4.4: Resources in Array/Dict ignored (Godot bug #74918).
        verify_duplication(&orig, &copy, false, false, true, true);
    }
}

#[cfg(since_api = "4.5")]
mod godot_4_5_tests {
    use godot::classes::resource::DeepDuplicateMode;

    use super::*;

    #[itest]
    fn duplicate_resource_deep_none() {
        let orig = create_test_resource();
        let copy = orig
            .duplicate_resource_ex()
            .deep(DeepDuplicateMode::NONE)
            .done();

        // All subresources shared, but array/dict are deep-copied.
        verify_duplication(&orig, &copy, false, false, false, true);
    }

    #[itest]
    fn duplicate_resource_deep_internal_mode() {
        let orig = create_test_resource();
        let copy = orig
            .duplicate_resource_ex()
            .deep(DeepDuplicateMode::INTERNAL)
            .done();

        // Internal duplicated, external shared.
        verify_duplication(&orig, &copy, true, false, true, true);
    }

    #[itest]
    fn duplicate_resource_deep_all() {
        let orig = create_test_resource();
        let copy = orig
            .duplicate_resource_ex()
            .deep(DeepDuplicateMode::ALL)
            .done();

        // All subresources duplicated.
        verify_duplication(&orig, &copy, true, true, true, true);
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Polymorphic duplication failure tests

#[derive(GodotClass)]
#[class(no_init, base=Node)]
struct NoInitNode {}

#[itest]
fn duplicate_node_no_init_fails() {
    // For some reason, this crashes for only API 4.3, and only when running with Godot 4.3. Newer versions are fine.
    #[cfg(all(since_api = "4.3", not(since_api = "4.4")))]
    if GdextBuild::godot_runtime_version_triple().1 == 3 {
        godot_warn!("Skip itest on Godot 4.3 runtime due to crash");
        return;
    }

    let node_ptr: Gd<Node> = Gd::from_object(NoInitNode {}).upcast();

    expect_panic("no_init node duplication", || {
        let _copy = node_ptr.duplicate_node();
    });

    node_ptr.free();
}

#[derive(GodotClass)]
#[class(no_init, base=Resource)]
struct NoInitResource {}

#[itest]
fn duplicate_resource_no_init_fails() {
    let resource_ptr: Gd<Resource> = Gd::from_object(NoInitResource {}).upcast();

    expect_panic("no_init resource duplication", || {
        let _copy = resource_ptr.duplicate_resource();
    });
}
