/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

pub mod registration;

mod class_name;
mod signature;

pub use class_name::*;
pub use signature::*;

use crate::builtin::*;
use crate::engine::global;

use godot_ffi as sys;

use registration::method::MethodParamOrReturnInfo;

/// Stores meta-information about registered types or properties.
///
/// Filling this information properly is important so that Godot can use ptrcalls instead of varcalls
/// (requires typed GDScript + sufficient information from the extension side)
pub trait VariantMetadata {
    fn variant_type() -> VariantType;

    fn class_name() -> ClassName {
        // If we use `ClassName::of::<()>()` then this type shows up as `(no base)` in documentation.
        ClassName::none()
    }

    fn property_info(property_name: &str) -> PropertyInfo {
        PropertyInfo {
            variant_type: Self::variant_type(),
            class_name: Self::class_name(),
            property_name: StringName::from(property_name),
            hint: global::PropertyHint::PROPERTY_HINT_NONE,
            hint_string: GodotString::new(),
            usage: global::PropertyUsageFlags::PROPERTY_USAGE_DEFAULT,
        }
    }

    fn param_metadata() -> sys::GDExtensionClassMethodArgumentMetadata {
        sys::GDEXTENSION_METHOD_ARGUMENT_METADATA_NONE
    }

    fn argument_info(property_name: &str) -> MethodParamOrReturnInfo {
        MethodParamOrReturnInfo::new(Self::property_info(property_name), Self::param_metadata())
    }

    fn return_info() -> Option<MethodParamOrReturnInfo> {
        Some(MethodParamOrReturnInfo::new(
            Self::property_info(""),
            Self::param_metadata(),
        ))
    }
}

impl<T: VariantMetadata> VariantMetadata for Option<T> {
    fn variant_type() -> VariantType {
        T::variant_type()
    }

    fn class_name() -> ClassName {
        T::class_name()
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Rusty abstraction of sys::GDExtensionPropertyInfo
/// Keeps the actual allocated values (the sys equivalent only keeps pointers, which fall out of scope)
#[derive(Debug)]
// Note: is not #[non_exhaustive], so adding fields is a breaking change. Mostly used internally at the moment though.
pub struct PropertyInfo {
    pub variant_type: VariantType,
    pub class_name: ClassName,
    pub property_name: StringName,
    pub hint: global::PropertyHint,
    pub hint_string: GodotString,
    pub usage: global::PropertyUsageFlags,
}

impl PropertyInfo {
    /// Converts to the FFI type. Keep this object allocated while using that!
    pub fn property_sys(&self) -> sys::GDExtensionPropertyInfo {
        use crate::obj::EngineEnum as _;

        sys::GDExtensionPropertyInfo {
            type_: self.variant_type.sys(),
            name: self.property_name.string_sys(),
            class_name: self.class_name.string_sys(),
            hint: u32::try_from(self.hint.ord()).expect("hint.ord()"),
            hint_string: self.hint_string.string_sys(),
            usage: u32::try_from(self.usage.ord()).expect("usage.ord()"),
        }
    }

    pub fn empty_sys() -> sys::GDExtensionPropertyInfo {
        use crate::obj::EngineEnum as _;

        sys::GDExtensionPropertyInfo {
            type_: VariantType::Nil.sys(),
            name: std::ptr::null_mut(),
            class_name: std::ptr::null_mut(),
            hint: global::PropertyHint::PROPERTY_HINT_NONE.ord() as u32,
            hint_string: std::ptr::null_mut(),
            usage: global::PropertyUsageFlags::PROPERTY_USAGE_NONE.ord() as u32,
        }
    }
}
