/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot_ffi::conv::u32_to_usize;

use crate::builtin::{StringName, Variant};
use crate::global::MethodFlags;
use crate::meta::{ClassId, PropertyInfo};
use crate::sys;

/// Describes a method in Godot.
///
/// Abstraction of the low-level `sys::GDExtensionMethodInfo`.
// Currently used for ScriptInstance.
// TODO check overlap with (private) ClassMethodInfo.
#[derive(Clone, Debug)]
pub struct MethodInfo {
    pub id: i32,
    pub method_name: StringName,
    pub class_name: ClassId,
    pub return_type: PropertyInfo,
    pub arguments: Vec<PropertyInfo>,
    /// Whether default arguments are real "arguments" is controversial. From the function PoV they are, but for the caller,
    /// they are just pre-set values to fill in for missing arguments.
    pub default_arguments: Vec<Variant>,
    pub flags: MethodFlags,
}

impl MethodInfo {
    /// Consumes self and turns it into a `sys::GDExtensionMethodInfo`, should be used together with
    /// [`free_owned_method_sys`](Self::free_owned_method_sys).
    ///
    /// This will leak memory unless used together with `free_owned_method_sys`.
    pub fn into_owned_method_sys(self) -> sys::GDExtensionMethodInfo {
        use crate::obj::EngineBitfield as _;

        // Destructure self to ensure all fields are used.
        let Self {
            id,
            method_name,
            // TODO: Do we need this?
            class_name: _class_name,
            return_type,
            arguments,
            default_arguments,
            flags,
        } = self;

        let argument_count: u32 = arguments
            .len()
            .try_into()
            .expect("cannot have more than `u32::MAX` arguments");
        let arguments = arguments
            .into_iter()
            .map(|arg| arg.into_owned_property_sys())
            .collect::<Box<[_]>>();
        let arguments = Box::leak(arguments).as_mut_ptr();

        let default_argument_count: u32 = default_arguments
            .len()
            .try_into()
            .expect("cannot have more than `u32::MAX` default arguments");
        let default_argument = default_arguments
            .into_iter()
            .map(|arg| arg.into_owned_var_sys())
            .collect::<Box<[_]>>();
        let default_arguments = Box::leak(default_argument).as_mut_ptr();

        sys::GDExtensionMethodInfo {
            id,
            name: method_name.into_owned_string_sys(),
            return_value: return_type.into_owned_property_sys(),
            argument_count,
            arguments,
            default_argument_count,
            default_arguments,
            flags: flags.ord().try_into().expect("flags should be valid"),
        }
    }

    /// Properly frees a `sys::GDExtensionMethodInfo` created by [`into_owned_method_sys`](Self::into_owned_method_sys).
    ///
    /// # Safety
    ///
    /// * Must only be used on a struct returned from a call to `into_owned_method_sys`, without modification.
    /// * Must not be called more than once on a `sys::GDExtensionMethodInfo` struct.
    #[deny(unsafe_op_in_unsafe_fn)]
    pub unsafe fn free_owned_method_sys(info: sys::GDExtensionMethodInfo) {
        // Destructure info to ensure all fields are used.
        let sys::GDExtensionMethodInfo {
            name,
            return_value,
            flags: _flags,
            id: _id,
            argument_count,
            arguments,
            default_argument_count,
            default_arguments,
        } = info;

        // SAFETY: `name` is a pointer that was returned from `StringName::into_owned_string_sys`, and has not been freed before this.
        let _name = unsafe { StringName::from_owned_string_sys(name) };

        // SAFETY: `return_value` is a pointer that was returned from `PropertyInfo::into_owned_property_sys`, and has not been freed before
        // this.
        unsafe { PropertyInfo::free_owned_property_sys(return_value) };

        // SAFETY:
        // - `from_raw_parts_mut`: `arguments` comes from `as_mut_ptr()` on a mutable slice of length `argument_count`, and no other
        //    accesses to the pointer happens for the lifetime of the slice.
        // - `Box::from_raw`: The slice was returned from a call to `Box::leak`, and we have ownership of the value behind this pointer.
        let arguments = unsafe {
            let slice = std::slice::from_raw_parts_mut(arguments, u32_to_usize(argument_count));

            Box::from_raw(slice)
        };

        for info in arguments.iter() {
            // SAFETY: These infos were originally created from a call to `PropertyInfo::into_owned_property_sys`, and this method
            // will not be called again on this pointer.
            unsafe { PropertyInfo::free_owned_property_sys(*info) }
        }

        // SAFETY:
        // - `from_raw_parts_mut`: `default_arguments` comes from `as_mut_ptr()` on a mutable slice of length `default_argument_count`, and no
        //    other accesses to the pointer happens for the lifetime of the slice.
        // - `Box::from_raw`: The slice was returned from a call to `Box::leak`, and we have ownership of the value behind this pointer.
        let default_arguments = unsafe {
            let slice = std::slice::from_raw_parts_mut(
                default_arguments,
                u32_to_usize(default_argument_count),
            );

            Box::from_raw(slice)
        };

        for variant in default_arguments.iter() {
            // SAFETY: These pointers were originally created from a call to `Variant::into_owned_var_sys`, and this method will not be
            // called again on this pointer.
            let _variant = unsafe { Variant::from_owned_var_sys(*variant) };
        }
    }
}
