/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot_ffi as sys;
use sys::interface_fn;

use crate::builtin::meta::{ClassName, PropertyInfo, VarcallSignatureTuple};
use crate::builtin::{StringName, Variant};
use crate::engine::global::MethodFlags;

/// Info relating to an argument or return type in a method.
pub struct MethodParamOrReturnInfo {
    info: PropertyInfo,
    metadata: sys::GDExtensionClassMethodArgumentMetadata,
}

impl MethodParamOrReturnInfo {
    pub fn new(info: PropertyInfo, metadata: sys::GDExtensionClassMethodArgumentMetadata) -> Self {
        Self { info, metadata }
    }
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
    /// # Safety
    ///
    /// `ptrcall_func`, if provided, must:
    ///
    /// - Interpret its parameters according to the types specified in `S`.
    /// - Return the value that is specified in `S`, or return nothing if the return value is `()`.
    ///
    /// `call_func`, if provided, must:
    ///
    /// - Interpret its parameters as a list of `S::PARAM_COUNT` `Variant`s.
    /// - Return a `Variant`.
    ///
    /// `call_func` and `ptrcall_func`, if provided, must:
    ///
    /// - Follow the behavior expected from the `method_flags`.
    pub unsafe fn from_signature<S: VarcallSignatureTuple>(
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
            return_value_info: std::ptr::addr_of_mut!(return_value_sys),
            return_value_metadata,
            argument_count: self.argument_count(),
            arguments_info: arguments_info_sys.as_mut_ptr(),
            arguments_metadata: arguments_metadata.as_mut_ptr(),
            default_argument_count: self.default_argument_count(),
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

    fn argument_count(&self) -> u32 {
        self.arguments
            .len()
            .try_into()
            .expect("arguments length should fit in u32")
    }

    fn default_argument_count(&self) -> u32 {
        self.default_arguments
            .len()
            .try_into()
            .expect("arguments length should fit in u32")
    }
}
