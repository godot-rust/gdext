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

/// Describes a method's signature and metadata required by the Godot engine.
///
/// Primarily used when implementing custom script instances via the [`ScriptInstance`][crate::obj::script::ScriptInstance] trait.
/// It contains metadata Godot needs to describe and call a method.
///
/// `MethodInfo` is a high-level abstraction over the low-level FFI type `sys::GDExtensionMethodInfo`.
///
/// See also [`PropertyInfo`] for describing individual method parameters and return types.
///
/// # Example
/// ```no_run
/// use godot::meta::{MethodInfo, PropertyInfo, PropertyHintInfo, ClassId};
/// use godot::builtin::{StringName, Variant, VariantType};
/// use godot::global::{MethodFlags, PropertyUsageFlags};
/// use godot::classes::Node2D;
/// use godot::obj::GodotClass; // Trait method ::class_id().
///
/// // Describe a Godot method (`World` is a GDScript class):
/// //   func spawn_at(world: World, position: Vector2) -> Node2D.
/// let method = MethodInfo {
///     id: 0,
///     method_name: StringName::from("spawn_at"),
///     class_name: ClassId::none(),
///     return_type: PropertyInfo {
///         variant_type: VariantType::OBJECT,
///         class_id: Node2D::class_id(),
///         property_name: StringName::default(), // Return types use empty string.
///         hint_info: PropertyHintInfo::none(),
///         usage: PropertyUsageFlags::DEFAULT,
///     },
///     arguments: vec![
///         PropertyInfo {
///             variant_type: VariantType::OBJECT,
///             class_id: ClassId::new_dynamic("World"),
///             property_name: StringName::from("world"),
///             hint_info: PropertyHintInfo::none(),
///             usage: PropertyUsageFlags::DEFAULT,
///         },
///         PropertyInfo {
///             variant_type: VariantType::VECTOR2,
///             class_id: ClassId::none(),
///             property_name: StringName::from("position"),
///             hint_info: PropertyHintInfo::none(),
///             usage: PropertyUsageFlags::DEFAULT,
///         },
///     ],
///     default_arguments: vec![],
///     flags: MethodFlags::DEFAULT,
/// };
/// ```
#[derive(Clone, Debug)]
pub struct MethodInfo {
    /// Unique identifier for the method within its class.
    ///
    /// This ID can be used to distinguish between methods and is typically set by the implementation. For script instances,
    /// this is often just a sequential index.
    pub id: i32,

    /// The name of the method, as it appears in Godot.
    pub method_name: StringName,

    /// The class this method belongs to.
    ///
    /// For script-defined methods, this is typically the script's class ID obtained via [`ClassId::new_dynamic()`].
    /// Use [`ClassId::none()`] if the class is not applicable or unknown.
    pub class_name: ClassId,

    /// Description of the method's return type.
    ///
    /// See [`PropertyInfo`] for how to construct type information. For methods that
    /// don't return a value (void), use `VariantType::NIL`.
    pub return_type: PropertyInfo,

    /// Descriptions of each method parameter.
    ///
    /// Each element describes one parameter's type, name, and metadata. The order
    /// matches the parameter order in the method signature.
    pub arguments: Vec<PropertyInfo>,

    /// Default values for parameters with defaults.
    ///
    /// Contains the actual default [`Variant`] values for parameters that have them.
    /// The length of this vector is typically less than or equal to `arguments.len()`,
    /// containing defaults only for trailing parameters.
    pub default_arguments: Vec<Variant>,

    /// Method flags controlling behavior and access.
    ///
    /// See [`MethodFlags`] for available options like `NORMAL`, `VIRTUAL`, `CONST`, etc.
    pub flags: MethodFlags,
}

impl MethodInfo {
    /// Consumes self and turns it into a `sys::GDExtensionMethodInfo`, should be used together with
    /// [`free_owned_method_sys`](Self::free_owned_method_sys).
    ///
    /// This will leak memory unless used together with `free_owned_method_sys`.
    #[doc(hidden)]
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
    #[doc(hidden)]
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
