/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

mod class_name;
mod signature;

pub use class_name::*;
pub use signature::*;
use sys::interface_fn;

use crate::builtin::*;
use crate::engine::global::{self, MethodFlags};

use godot_ffi as sys;

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
        MethodParamOrReturnInfo {
            info: Self::property_info(property_name),
            metadata: Self::param_metadata(),
        }
    }

    fn return_info() -> Option<MethodParamOrReturnInfo> {
        Some(MethodParamOrReturnInfo {
            info: Self::property_info(""),
            metadata: Self::param_metadata(),
        })
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

// ----------------------------------------------------------------------------------------------------------------------------------------------
/// Rusty abstraction of sys::GDExtensionClassMethodInfo

/// Info relating to an argument or return type in a method.
pub struct MethodParamOrReturnInfo {
    info: PropertyInfo,
    metadata: sys::GDExtensionClassMethodArgumentMetadata,
}

/// All info needed to register a method for a class with Godot.
pub struct MethodInfo {
    class_name: ClassName,
    method_name: StringName,
    call_func: sys::GDExtensionClassMethodCall,
    ptrcall_func: sys::GDExtensionClassMethodPtrCall,
    method_flags: MethodFlags,
    return_value: Option<MethodParamOrReturnInfo>,
    arguments: Vec<MethodParamOrReturnInfo>,
    default_arguments: Vec<Variant>,
}

impl MethodInfo {
    pub fn from_signature<S: VarcallSignatureTuple>(
        class_name: ClassName,
        method_name: StringName,
        call_func: sys::GDExtensionClassMethodCall,
        ptrcall_func: sys::GDExtensionClassMethodPtrCall,
        method_flags: MethodFlags,
        param_names: &[&str],
        default_arguments: Vec<Variant>,
    ) -> Self {
        let return_value = S::return_info();
        let mut arguments = Vec::new();

        assert_eq!(
            param_names.len(),
            S::PARAM_COUNT,
            "`param_names` should contain one name for each parameter"
        );

        for (i, name) in param_names.iter().enumerate().take(S::PARAM_COUNT) {
            arguments.push(S::param_info(i, name).unwrap_or_else(|| {
                panic!(
                    "signature with `PARAM_COUNT = {}` should have argument info for index `{i}`",
                    S::PARAM_COUNT
                )
            }))
        }

        assert!(
            default_arguments.len() <= arguments.len(),
            "cannot have more default arguments than arguments"
        );

        Self {
            class_name,
            method_name,
            call_func,
            ptrcall_func,
            method_flags,
            return_value,
            arguments,
            default_arguments,
        }
    }

    pub fn register_extension_class_method(&self) {
        use crate::obj::EngineEnum as _;

        let (return_value_info, return_value_metadata) = match &self.return_value {
            Some(info) => (Some(&info.info), info.metadata),
            None => (None, 0),
        };

        let mut return_value_sys = return_value_info
            .as_ref()
            .map(|info| info.property_sys())
            .unwrap_or(PropertyInfo::empty_sys());

        let mut arguments_info_sys: Vec<sys::GDExtensionPropertyInfo> = self
            .arguments
            .iter()
            .map(|argument| argument.info.property_sys())
            .collect();

        let mut arguments_metadata: Vec<sys::GDExtensionClassMethodArgumentMetadata> =
            self.arguments.iter().map(|info| info.metadata).collect();

        let mut default_arguments_sys: Vec<sys::GDExtensionVariantPtr> =
            self.default_arguments.iter().map(|v| v.var_sys()).collect();

        let method_info_sys = sys::GDExtensionClassMethodInfo {
            name: self.method_name.string_sys(),
            method_userdata: std::ptr::null_mut(),
            call_func: self.call_func,
            ptrcall_func: self.ptrcall_func,
            method_flags: self.method_flags.ord() as u32,
            has_return_value: self.return_value.is_some() as u8,
            return_value_info: &mut return_value_sys as *mut sys::GDExtensionPropertyInfo,
            return_value_metadata,
            argument_count: self
                .arguments
                .len()
                .try_into()
                .expect("arguments length should fit in u32"),
            arguments_info: arguments_info_sys.as_mut_ptr(),
            arguments_metadata: arguments_metadata.as_mut_ptr(),
            default_argument_count: self
                .default_arguments
                .len()
                .try_into()
                .expect("default arguments length should fit in u32"),
            default_arguments: default_arguments_sys.as_mut_ptr(),
        };
        // SAFETY:
        // The lifetime of the data we use here is at least as long as this function's scope. So we can
        // safely call this function without issue.
        //
        // Null pointers will only be passed along if we indicate to Godot that they are unused.
        unsafe {
            interface_fn!(classdb_register_extension_class_method)(
                sys::get_library(),
                self.class_name.string_sys(),
                std::ptr::addr_of!(method_info_sys),
            )
        }
    }
}
