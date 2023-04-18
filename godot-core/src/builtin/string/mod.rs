/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Godot-types that are Strings.

mod godot_string;
mod macros;
mod node_path;
mod string_chars;
mod string_name;

use godot_ffi::VariantType;
pub use godot_string::*;
pub use node_path::*;
pub use string_name::*;

use super::{meta::VariantMetadata, FromVariant, ToVariant, Variant, VariantConversionError};

impl ToVariant for &str {
    fn to_variant(&self) -> Variant {
        GodotString::from(*self).to_variant()
    }
}

impl ToVariant for String {
    fn to_variant(&self) -> Variant {
        GodotString::from(self).to_variant()
    }
}

impl FromVariant for String {
    fn try_from_variant(variant: &Variant) -> Result<Self, VariantConversionError> {
        Ok(GodotString::try_from_variant(variant)?.to_string())
    }
}

impl VariantMetadata for String {
    fn variant_type() -> VariantType {
        VariantType::String
    }
}
