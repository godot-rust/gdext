/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::builtin::{StringName, VariantType};
use crate::meta::traits::GodotFfi;
use crate::meta::GodotType;
use std::fmt;

/// Represents the type information of a Godot array. See
/// [`set_typed`](https://docs.godotengine.org/en/latest/classes/class_array.html#class-array-method-set-typed).
///
/// We ignore the `script` parameter because it has no impact on typing in Godot.
#[derive(Eq, PartialEq)]
pub(crate) struct ArrayTypeInfo {
    pub variant_type: VariantType,

    /// Not a `ClassName` because some values come from Godot engine API.
    pub class_name: StringName,
}

impl ArrayTypeInfo {
    pub fn of<T: GodotType>() -> Self {
        Self {
            variant_type: <T::Via as GodotType>::Ffi::variant_type(),
            class_name: T::Via::class_name().to_string_name(),
        }
    }

    pub fn is_typed(&self) -> bool {
        self.variant_type != VariantType::NIL
    }

    pub fn variant_type(&self) -> VariantType {
        self.variant_type
    }

    pub fn class_name(&self) -> &StringName {
        &self.class_name
    }
}

impl fmt::Debug for ArrayTypeInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let class = self.class_name.to_string();
        let class_str = if class.is_empty() {
            String::new()
        } else {
            format!(" (class={class})")
        };

        write!(f, "{:?}{}", self.variant_type, class_str)
    }
}
