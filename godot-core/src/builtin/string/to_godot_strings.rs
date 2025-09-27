/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::builtin::{GString, NodePath, StringName};

/// Extension trait for converting to Godot string types.
///
/// This trait provides a uniform interface for converting from Rust string types `&str` and `String`, to all three Godot string types
/// `GString`, `StringName`, and `NodePath`.
pub trait ToGodotStrings {
    /// Convert to a `GString`.
    fn to_gstring(&self) -> GString;

    /// Convert to a `StringName`.
    fn to_string_name(&self) -> StringName;

    /// Convert to a `NodePath`.
    fn to_node_path(&self) -> NodePath;
}

impl ToGodotStrings for &str {
    fn to_gstring(&self) -> GString {
        GString::from(*self)
    }

    fn to_string_name(&self) -> StringName {
        StringName::from(*self)
    }

    fn to_node_path(&self) -> NodePath {
        NodePath::from(*self)
    }
}

impl ToGodotStrings for String {
    fn to_gstring(&self) -> GString {
        GString::from(self)
    }

    fn to_string_name(&self) -> StringName {
        StringName::from(self)
    }

    fn to_node_path(&self) -> NodePath {
        NodePath::from(self)
    }
}
