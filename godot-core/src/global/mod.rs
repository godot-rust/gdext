/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Godot global enums, constants and utility functions.
//!
//! See also [Godot docs for `@GlobalScope`](https://docs.godotengine.org/en/stable/classes/class_@globalscope.html#methods).
//!
//! # Builtin-related enums
//!
//! The library ships several additional enums in places where GDScript would use magic numbers. These are co-located with
//! builtin types, in the [`godot::builtin`][crate::builtin] module. The enums are:
//!
//! - Color: [`ColorChannelOrder`][crate::builtin::ColorChannelOrder]
//! - Projection: [`ProjectionEye`][crate::builtin::ProjectionEye], [`ProjectionPlane`][crate::builtin::ProjectionPlane]
//! - Rectangle: [`Side`], [`Corner`] <sub>(godot-generated)</sub>
//! - Rotation: [`EulerOrder`] <sub>(godot-generated)</sub>
//! - Variant: [`VariantType`][crate::builtin::VariantType], [`VariantOperator`][crate::builtin::VariantOperator]
//! - Vector: [`Vector2Axis`][crate::builtin::Vector2Axis], [`Vector3Axis`][crate::builtin::Vector3Axis], [`Vector4Axis`][crate::builtin::Vector4Axis]
//!
//! # Functions moved to dedicated APIs
//!
//! Some methods in `@GlobalScope` are not directly available in `godot::global` module, but rather in their related types.  \
//! You can find them as follows:
//!
//! | Godot utility function | godot-rust APIs                                                                                                                      |
//! |------------------------|--------------------------------------------------------------------------------------------------------------------------------------|
//! | `instance_from_id`     | [`Gd::from_instance_id()`][crate::obj::Gd::from_instance_id]<br>[`Gd::try_from_instance_id()`][crate::obj::Gd::try_from_instance_id()] |
//! | `is_instance_valid`    | [`Gd::is_instance_valid()`][crate::obj::Gd::is_instance_valid()]                                                                     |
//! | `is_instance_id_valid` | [`InstanceId::lookup_validity()`][crate::obj::InstanceId::lookup_validity()]                                                         |
//!

// Doc aliases are also available in dedicated APIs, but directing people here may give them a bit more context.
#![doc(
    alias = "instance_from_id",
    alias = "is_instance_valid",
    alias = "is_instance_id_valid"
)]

mod print;

pub use crate::{
    godot_error, godot_print, godot_print_rich, godot_script_error, godot_str, godot_warn,
};

// Some enums are directly re-exported from crate::builtin.
pub use crate::gen::central::global_enums::*;
pub use crate::gen::utilities::*;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Internal re-exports

// This is needed for generated classes to find symbols, even those that have been moved to crate::builtin.
#[allow(unused_imports)] // micromanaging imports for generated code is not fun
pub(crate) use crate::builtin::{Corner, EulerOrder, Side};

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Deprecations
