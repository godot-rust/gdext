/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::fmt;

use crate::builtin::{StringName, VariantType};
use crate::meta::traits::element_variant_type;
use crate::meta::{ArrayElement, GodotType};

/// Represents the type information of a Godot array. See
/// [`set_typed`](https://docs.godotengine.org/en/latest/classes/class_array.html#class-array-method-set-typed).
///
/// We ignore the `script` parameter because it has no impact on typing in Godot.
#[derive(Eq, PartialEq)]
pub(crate) struct ArrayTypeInfo {
    /// The builtin type; always set.
    pub variant_type: VariantType,

    /// If [`variant_type`] is [`VariantType::OBJECT`], then the class name; otherwise `None`.
    ///
    /// Not a `ClassName` because some values come from Godot engine API.
    pub class_name: Option<StringName>,
}

impl ArrayTypeInfo {
    pub fn of<T: ArrayElement>() -> Self {
        let variant_type = element_variant_type::<T>();
        let class_name = if variant_type == VariantType::OBJECT {
            Some(T::Via::class_name().to_string_name())
        } else {
            None
        };

        Self {
            variant_type,
            class_name,
        }
    }

    pub fn is_typed(&self) -> bool {
        self.variant_type != VariantType::NIL
    }

    pub fn variant_type(&self) -> VariantType {
        self.variant_type
    }

    pub fn class_name(&self) -> Option<&StringName> {
        self.class_name.as_ref()
    }
}

impl fmt::Debug for ArrayTypeInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let class_str = if let Some(class) = &self.class_name {
            format!(" (class={class})")
        } else {
            String::new()
        };

        write!(f, "{:?}{}", self.variant_type, class_str)
    }
}
