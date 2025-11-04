/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot_ffi as sys;
use sys::interface_fn;

use crate::builtin::{StringName, Variant};
use crate::global::MethodFlags;
use crate::meta::{ClassId, GodotConvert, GodotType, ParamTuple, PropertyInfo, Signature};
use crate::obj::GodotClass;

/// Info relating to an argument or return type in a method.
pub struct MethodParamOrReturnInfo {
    pub(crate) info: PropertyInfo,
    metadata: sys::GDExtensionClassMethodArgumentMetadata,
}

impl MethodParamOrReturnInfo {
    pub fn new(info: PropertyInfo, metadata: sys::GDExtensionClassMethodArgumentMetadata) -> Self {
        Self { info, metadata }
    }
}

/// All info needed to register a method for a class with Godot.
pub struct ClassMethodInfo {
    class_id: ClassId,
    method_name: StringName,
    call_func: sys::GDExtensionClassMethodCall,
    ptrcall_func: sys::GDExtensionClassMethodPtrCall,
    method_flags: MethodFlags,
    return_value: Option<MethodParamOrReturnInfo>,
    arguments: Vec<MethodParamOrReturnInfo>,
    /// Whether default arguments are real "arguments" is controversial. From the function PoV they are, but for the caller,
    /// they are just pre-set values to fill in for missing arguments.
    default_arguments: Vec<Variant>,
}

impl ClassMethodInfo {
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
    pub unsafe fn from_signature<C: GodotClass, Params: ParamTuple, Ret: GodotConvert>(
        method_name: StringName,
        call_func: sys::GDExtensionClassMethodCall,
        ptrcall_func: sys::GDExtensionClassMethodPtrCall,
        method_flags: MethodFlags,
        param_names: &[&str],
        default_arguments: Vec<Variant>,
    ) -> Self {
        let return_value = Ret::Via::return_info();
        let arguments = Signature::<Params, Ret>::param_names(param_names);

        assert!(
            default_arguments.len() <= arguments.len(),
            "cannot have more default arguments than arguments"
        );

        Self {
            class_id: C::class_id(),
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
        use crate::obj::EngineBitfield as _;

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

        let mut default_arguments_sys: Vec<sys::GDExtensionVariantPtr> = self
            .default_arguments
            .iter()
            .map(|v| sys::SysPtr::force_mut(v.var_sys()))
            .collect();

        let method_info_sys = sys::GDExtensionClassMethodInfo {
            name: sys::SysPtr::force_mut(self.method_name.string_sys()),
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

        if self.method_flags.is_set(MethodFlags::VIRTUAL) {
            self.register_virtual_class_method(method_info_sys, return_value_sys);
        } else {
            self.register_nonvirtual_class_method(method_info_sys);
        }
    }

    fn register_nonvirtual_class_method(&self, method_info_sys: sys::GDExtensionClassMethodInfo) {
        // SAFETY: The lifetime of the data we use here is at least as long as this function's scope. So we can
        // safely call this function without issue.
        //
        // Null pointers will only be passed along if we indicate to Godot that they are unused.
        unsafe {
            interface_fn!(classdb_register_extension_class_method)(
                sys::get_library(),
                self.class_id.string_sys(),
                std::ptr::addr_of!(method_info_sys),
            )
        }
    }

    #[cfg(since_api = "4.3")]
    fn register_virtual_class_method(
        &self,
        normal_method_info: sys::GDExtensionClassMethodInfo,
        return_value_sys: sys::GDExtensionPropertyInfo, // passed separately because value, not pointer.
    ) {
        // Copy everything possible from regular method info.
        let method_info_sys = sys::GDExtensionClassVirtualMethodInfo {
            name: normal_method_info.name,
            method_flags: normal_method_info.method_flags,
            return_value: return_value_sys,
            return_value_metadata: normal_method_info.return_value_metadata,
            argument_count: normal_method_info.argument_count,
            arguments: normal_method_info.arguments_info,
            arguments_metadata: normal_method_info.arguments_metadata,
        };

        // SAFETY: Godot only needs arguments to be alive during the method call.
        unsafe {
            interface_fn!(classdb_register_extension_class_virtual_method)(
                sys::get_library(),
                self.class_id.string_sys(),
                std::ptr::addr_of!(method_info_sys),
            )
        }
    }

    // Polyfill doing nothing.
    #[cfg(before_api = "4.3")]
    fn register_virtual_class_method(
        &self,
        _normal_method_info: sys::GDExtensionClassMethodInfo,
        _return_value_sys: sys::GDExtensionPropertyInfo,
    ) {
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
