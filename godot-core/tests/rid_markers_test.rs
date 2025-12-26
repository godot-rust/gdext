/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Test that RID marker types are accessible and work as expected.

use godot::classes::rids::*;

#[test]
fn rid_markers_are_copy() {
    let tag = TagCanvas;
    let _tag2 = tag; // Should be Copy
    let _tag3 = tag; // Can use again
}

#[test]
fn rid_markers_are_eq() {
    assert_eq!(TagCanvas, TagCanvas);
    assert_eq!(TagPhysicsSpace2D, TagPhysicsSpace2D);
    assert_eq!(TagNavigationMap, TagNavigationMap);
}

#[test]
fn rid_markers_are_debug() {
    let debug_str = format!("{:?}", TagScenario);
    assert_eq!(debug_str, "TagScenario");
}

#[test]
fn all_rendering_markers_exist() {
    let _: TagCanvas = TagCanvas;
    let _: TagScenario = TagScenario;
    let _: TagInstance = TagInstance;
    let _: TagSkeleton = TagSkeleton;
    let _: TagOccluder = TagOccluder;
    let _: TagLightmap = TagLightmap;
    let _: TagCompositor = TagCompositor;
    let _: TagCompositorEffect = TagCompositorEffect;
    let _: TagViewportRid = TagViewportRid;
}

#[test]
fn all_physics_markers_exist() {
    // 2D
    let _: TagPhysicsSpace2D = TagPhysicsSpace2D;
    let _: TagPhysicsArea2D = TagPhysicsArea2D;
    let _: TagPhysicsBody2D = TagPhysicsBody2D;
    let _: TagPhysicsJoint2D = TagPhysicsJoint2D;
    
    // 3D
    let _: TagPhysicsSpace3D = TagPhysicsSpace3D;
    let _: TagPhysicsArea3D = TagPhysicsArea3D;
    let _: TagPhysicsBody3D = TagPhysicsBody3D;
    let _: TagPhysicsSoftBody3D = TagPhysicsSoftBody3D;
    let _: TagPhysicsJoint3D = TagPhysicsJoint3D;
}

#[test]
fn all_navigation_markers_exist() {
    let _: TagNavigationMap = TagNavigationMap;
    let _: TagNavigationRegion = TagNavigationRegion;
    let _: TagNavigationLink = TagNavigationLink;
    let _: TagNavigationAgent = TagNavigationAgent;
    let _: TagNavigationObstacle = TagNavigationObstacle;
    let _: TagNavigationParser = TagNavigationParser;
}

#[test]
fn all_other_markers_exist() {
    let _: TagShapedText = TagShapedText;
    let _: TagAccessibilityElement = TagAccessibilityElement;
}
