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
//! - Rectangle: [`Side`][crate::builtin::Side], [`Corner`][crate::builtin::Corner] <sub>(godot-generated)</sub>
//! - Rotation: [`EulerOrder`][crate::builtin::EulerOrder] <sub>(godot-generated)</sub>
//! - Variant: [`VariantType`][crate::builtin::VariantType], [`VariantOperator`][crate::builtin::VariantOperator]
//! - Vector: [`Vector2Axis`][crate::builtin::Vector2Axis], [`Vector3Axis`][crate::builtin::Vector3Axis], [`Vector4Axis`][crate::builtin::Vector4Axis]
//!

mod print;
mod save_load;

pub use crate::{godot_error, godot_print, godot_print_rich, godot_script_error, godot_warn};

// Some enums are directly re-exported from crate::builtin.
pub use crate::gen::central::global_enums::*;
pub use crate::gen::utilities::*;

pub use save_load::*;

// This is needed for generated classes to find symbols, even those that have been moved to crate::builtin.
#[allow(unused_imports)] // micromanaging imports for generated code is not fun
pub(crate) use crate::builtin::{Corner, EulerOrder, Side};
