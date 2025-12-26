/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! RID type markers for server-internal resource types.
//!
//! These marker types represent server-side resources that don't have corresponding
//! Godot class types. They are used with `TypedRid<T>` to provide type safety for
//! low-level server APIs.
//!
//! # Background
//!
//! Godot's server APIs (`RenderingServer`, `PhysicsServer2D`, `PhysicsServer3D`, etc.)
//! work with RIDs that represent internal resources. While some RIDs correspond to
//! scene tree classes (e.g., `Mesh`, `Shader`), many represent server-internal types
//! that have no Godot class equivalent.
//!
//! These markers allow type-safe RID usage for those server-internal types.

use crate::meta::sealed;

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// RenderingServer markers
// ══════════════════════════════════════════════════════════════════════════════════════════════════

/// Marker for RIDs returned by `RenderingServer::canvas_create()`.
///
/// Represents a canvas rendering context (not to be confused with [`CanvasItem`]).
///
/// [`CanvasItem`]: crate::classes::CanvasItem
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct TagCanvas;
impl sealed::Sealed for TagCanvas {}

/// Marker for RIDs returned by `RenderingServer::scenario_create()`.
///
/// Represents a 3D rendering scenario/world container.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct TagScenario;
impl sealed::Sealed for TagScenario {}

/// Marker for RIDs returned by `RenderingServer::instance_create()` and `instance_create2()`.
///
/// Represents a rendering instance in a scenario.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct TagInstance;
impl sealed::Sealed for TagInstance {}

/// Marker for RIDs returned by `RenderingServer::skeleton_create()`.
///
/// Represents a skeleton for mesh deformation (server-side).
/// Note: Different from the `Skeleton3D` scene tree node.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct TagSkeleton;
impl sealed::Sealed for TagSkeleton {}

/// Marker for RIDs returned by `RenderingServer::occluder_create()`.
///
/// Represents an occlusion culling occluder.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct TagOccluder;
impl sealed::Sealed for TagOccluder {}

/// Marker for RIDs returned by `RenderingServer::lightmap_create()`.
///
/// Represents a lightmap resource.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct TagLightmap;
impl sealed::Sealed for TagLightmap {}

/// Marker for RIDs returned by `RenderingServer::compositor_create()`.
///
/// Represents a compositor for custom rendering pipelines.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct TagCompositor;
impl sealed::Sealed for TagCompositor {}

/// Marker for RIDs returned by `RenderingServer::compositor_effect_create()`.
///
/// Represents a compositor effect.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct TagCompositorEffect;
impl sealed::Sealed for TagCompositorEffect {}

/// Marker for RIDs returned by `RenderingServer::viewport_create()`.
///
/// Represents a rendering viewport (server-side).
/// Note: Different from the [`Viewport`] scene tree node.
///
/// [`Viewport`]: crate::classes::Viewport
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct TagViewportRid;
impl sealed::Sealed for TagViewportRid {}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// PhysicsServer2D markers
// ══════════════════════════════════════════════════════════════════════════════════════════════════

/// Marker for RIDs returned by `PhysicsServer2D::space_create()`.
///
/// Represents a 2D physics space (server-side).
/// Note: Different from any scene tree node.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct TagPhysicsSpace2D;
impl sealed::Sealed for TagPhysicsSpace2D {}

/// Marker for RIDs returned by `PhysicsServer2D::area_create()`.
///
/// Represents a 2D physics area (server-side).
/// Note: Different from the `Area2D` scene tree node.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct TagPhysicsArea2D;
impl sealed::Sealed for TagPhysicsArea2D {}

/// Marker for RIDs returned by `PhysicsServer2D::body_create()`.
///
/// Represents a 2D physics body (server-side).
/// Note: Different from `PhysicsBody2D` and its subclasses.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct TagPhysicsBody2D;
impl sealed::Sealed for TagPhysicsBody2D {}

/// Marker for RIDs returned by `PhysicsServer2D::joint_create()`.
///
/// Represents a 2D physics joint (server-side).
/// Note: Different from `Joint2D` and its subclasses.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct TagPhysicsJoint2D;
impl sealed::Sealed for TagPhysicsJoint2D {}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// PhysicsServer3D markers
// ══════════════════════════════════════════════════════════════════════════════════════════════════

/// Marker for RIDs returned by `PhysicsServer3D::space_create()`.
///
/// Represents a 3D physics space (server-side).
/// Note: Different from any scene tree node.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct TagPhysicsSpace3D;
impl sealed::Sealed for TagPhysicsSpace3D {}

/// Marker for RIDs returned by `PhysicsServer3D::area_create()`.
///
/// Represents a 3D physics area (server-side).
/// Note: Different from the `Area3D` scene tree node.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct TagPhysicsArea3D;
impl sealed::Sealed for TagPhysicsArea3D {}

/// Marker for RIDs returned by `PhysicsServer3D::body_create()`.
///
/// Represents a 3D physics body (server-side).
/// Note: Different from `PhysicsBody3D` and its subclasses.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct TagPhysicsBody3D;
impl sealed::Sealed for TagPhysicsBody3D {}

/// Marker for RIDs returned by `PhysicsServer3D::soft_body_create()`.
///
/// Represents a 3D soft body (server-side).
/// Note: Different from the `SoftBody3D` scene tree node.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct TagPhysicsSoftBody3D;
impl sealed::Sealed for TagPhysicsSoftBody3D {}

/// Marker for RIDs returned by `PhysicsServer3D::joint_create()`.
///
/// Represents a 3D physics joint (server-side).
/// Note: Different from `Joint3D` and its subclasses.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct TagPhysicsJoint3D;
impl sealed::Sealed for TagPhysicsJoint3D {}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// NavigationServer markers (shared between 2D and 3D)
// ══════════════════════════════════════════════════════════════════════════════════════════════════

/// Marker for RIDs returned by `NavigationServer2D::map_create()` and `NavigationServer3D::map_create()`.
///
/// Represents a navigation map (server-side).
/// Note: No corresponding Godot class exists.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct TagNavigationMap;
impl sealed::Sealed for TagNavigationMap {}

/// Marker for RIDs returned by `NavigationServer2D::region_create()` and `NavigationServer3D::region_create()`.
///
/// Represents a navigation region (server-side).
/// Note: Different from `NavigationRegion2D`/`NavigationRegion3D` scene tree nodes.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct TagNavigationRegion;
impl sealed::Sealed for TagNavigationRegion {}

/// Marker for RIDs returned by `NavigationServer2D::link_create()` and `NavigationServer3D::link_create()`.
///
/// Represents a navigation link (server-side).
/// Note: Different from `NavigationLink2D`/`NavigationLink3D` scene tree nodes.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct TagNavigationLink;
impl sealed::Sealed for TagNavigationLink {}

/// Marker for RIDs returned by `NavigationServer2D::agent_create()` and `NavigationServer3D::agent_create()`.
///
/// Represents a navigation agent (server-side).
/// Note: Different from `NavigationAgent2D`/`NavigationAgent3D` scene tree nodes.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct TagNavigationAgent;
impl sealed::Sealed for TagNavigationAgent {}

/// Marker for RIDs returned by `NavigationServer2D::obstacle_create()` and `NavigationServer3D::obstacle_create()`.
///
/// Represents a navigation obstacle (server-side).
/// Note: Different from `NavigationObstacle2D`/`NavigationObstacle3D` scene tree nodes.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct TagNavigationObstacle;
impl sealed::Sealed for TagNavigationObstacle {}

/// Marker for RIDs returned by `NavigationServer2D::source_geometry_parser_create()` and
/// `NavigationServer3D::source_geometry_parser_create()`.
///
/// Represents a navigation mesh source geometry parser.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct TagNavigationParser;
impl sealed::Sealed for TagNavigationParser {}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// TextServer markers
// ══════════════════════════════════════════════════════════════════════════════════════════════════

/// Marker for RIDs returned by `TextServer::create_shaped_text()`.
///
/// Represents shaped text for complex text rendering.
/// Note: No corresponding Godot class exists.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct TagShapedText;
impl sealed::Sealed for TagShapedText {}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// DisplayServer markers
// ══════════════════════════════════════════════════════════════════════════════════════════════════

/// Marker for RIDs returned by accessibility-related methods in `DisplayServer`.
///
/// Represents an accessibility element in the platform's accessibility API.
/// Note: No corresponding Godot class exists.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct TagAccessibilityElement;
impl sealed::Sealed for TagAccessibilityElement {}
