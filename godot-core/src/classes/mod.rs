/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Maps the Godot class API to Rust.
//!
//! This module contains the following symbols:
//! * Classes: `CanvasItem`, etc.
//! * Interface traits: `ICanvasItem`, etc.
//! * Enum/flag modules: `canvas_item`, etc.
//!
//! Noteworthy sub-modules of `godot::classes` are:
//! * [`native`]: definition of _native structure_ types.
//! * [`notify`]: all notification enums, used when working with the virtual callback to handle lifecycle notifications.
//! * [`rids`]: type markers for server-internal RID types.

mod class_runtime;
mod manual_extensions;
mod match_class;
mod type_safe_replacements;

// Re-exports all generated classes, interface traits and sidecar modules.
pub use crate::r#gen::classes::*;
// Macro re-export.
pub use crate::match_class;

/// Support for Godot _native structures_.
///
/// Native structures are a niche API in Godot. These are low-level data types that are passed as pointers to/from the engine.
/// In Rust, they are represented as `#[repr(C)]` structs.
///
/// There is unfortunately not much official documentation available; you may need to look at Godot source code.
/// Most users will not need native structures, as they are very specialized.
pub mod native {
    pub use crate::r#gen::native::*;
}

/// Type markers for server-internal RID types.
///
/// These marker types represent server-side resources that don't have corresponding Godot class types.
/// They are intended for use with typed RID wrappers to provide type safety for low-level server APIs.
///
/// Examples include `CanvasTag`, `PhysicsServer2DSpaceTag`, `NavigationServer2DMapTag`, etc.
///
/// See the [module documentation](rid_tags) for details on each marker type.
pub use crate::r#gen::rid_tags as rids;
pub use crate::r#gen::rid_tags::*;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Crate-local utilities

pub(crate) use class_runtime::*;
