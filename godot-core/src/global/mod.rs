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
//! # Functions moved to dedicated APIs
//! Some methods in `@GlobalScope` are not directly available in `godot::global` module, but rather in their related types.  \
//! You can find them as follows:
//!
//! | Godot utility function | godot-rust APIs                                                                                                                        |
//! |------------------------|----------------------------------------------------------------------------------------------------------------------------------------|
//! | `instance_from_id`     | [`Gd::from_instance_id()`][crate::obj::Gd::from_instance_id]<br>[`Gd::try_from_instance_id()`][crate::obj::Gd::try_from_instance_id()] |
//! | `is_instance_valid`    | [`Gd::is_instance_valid()`][crate::obj::Gd::is_instance_valid()]                                                                       |
//! | `is_instance_id_valid` | [`InstanceId::lookup_validity()`][crate::obj::InstanceId::lookup_validity()]                                                           |
//!
//! # Global enums in other modules
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
//! Some enums are closely related to property/method registration and are located in [`godot::register::info`][crate::registry::info]:
//!
//! - [`PropertyHint`][crate::registry::info::PropertyHint] <sub>(godot-generated)</sub>
//! - [`PropertyUsageFlags`][crate::registry::info::PropertyUsageFlags] <sub>(godot-generated)</sub>
//! - [`MethodFlags`][crate::registry::info::MethodFlags] <sub>(godot-generated)</sub>

// Doc aliases are also available in dedicated APIs, but directing people here may give them a bit more context.
#![doc(
    alias = "instance_from_id",
    alias = "is_instance_valid",
    alias = "is_instance_id_valid"
)]

mod print;

pub use print::{__threadsafe_print, PrintLevel, PrintRecord, PrintSource, print_custom};

// Some enums are directly re-exported from crate::builtin.
pub use crate::r#gen::central::global_enums::*;
pub use crate::r#gen::utilities::*;
pub use crate::{
    godot_error, godot_print, godot_print_rich, godot_script_error, godot_str, godot_warn,
};

// Some enums that are global in Godot are moved to different Rust modules. Codegen takes care of it by adjusting their path accordingly.
// See get_global_enum_rust_path() in special_cases.rs.

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Deprecations
