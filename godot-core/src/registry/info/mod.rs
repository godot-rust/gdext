/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Metadata types for property and method registration.
//!
//! Provides Rust mappings of types used for GDExtension registration of properties, methods and their type metadata.
//! See also [`meta::shape`](crate::meta::shape) for the static type description of those.

mod method_info;
mod param_metadata;
mod property_info;

pub use self::method_info::MethodInfo;
pub use self::param_metadata::ParamMetadata;
pub use self::property_info::{PropertyHintInfo, PropertyInfo};
pub use crate::r#gen::central::global_reexported_enums::{
    MethodFlags, PropertyHint, PropertyUsageFlags,
};
