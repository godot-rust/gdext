/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Meta-information about variant types, properties and class names.
//!
//! # Conversions between types
//!
//! ## Godot representation
//!
//! The library provides two traits [`FromGodot`] and [`ToGodot`], which are used at the Rust <-> Godot boundary, both in user-defined functions
//! ([`#[func]`](../register/attr.godot_api.html#user-defined-functions)) and engine APIs ([`godot::classes` module](crate::classes)).
//! Their `to_godot()` and `from_godot()` methods convert types from/to their _closest possible Godot type_ (e.g. `GString` instead of Rust
//! `String`). You usually don't need to call these methods yourself, they are automatically invoked when passing objects to/from Godot.
//!
//! Most often, the two traits appear in pairs, however there are cases where only one of the two is implemented. For example, `&str` implements
//! `ToGodot` but not `FromGodot`. Additionally, [`GodotConvert`] acts as a supertrait of both [`FromGodot`] and [`ToGodot`]. Its sole purpose
//! is to define the "closest possible Godot type" [`GodotConvert::Via`].
//!
//! For fallible conversions, you can use [`FromGodot::try_from_godot()`].
//!
//! ## Variants
//!
//! [`ToGodot`] and [`FromGodot`] also implement a conversion to/from [`Variant`], which is the most versatile Godot type. This conversion is
//! available via `to_variant()` and `from_variant()` methods. These methods are also available directly on `Variant` itself, via `to()`,
//! `try_to()` and `from()` functions.
//!
//! ## Class conversions
//!
//! Godot classes exist in a hierarchy. In OOP, it is usually possible to represent pointers to derived objects as pointer to their bases.
//! For conversions between base and derived class objects, you can use `Gd` methods [`cast()`][crate::obj::Gd::cast],
//! [`try_cast()`][crate::obj::Gd::try_cast] and [`upcast()`][crate::obj::Gd::upcast]. Upcasts are infallible.

mod array_type_info;
mod class_name;
mod godot_convert;
mod sealed;
mod signature;
mod traits;

pub mod error;
pub use class_name::ClassName;
pub use godot_convert::{FromGodot, GodotConvert, ToGodot};
use sys::conv::u32_to_usize;
pub use traits::{ArrayElement, GodotType};

pub(crate) use crate::impl_godot_as_self;
pub(crate) use array_type_info::ArrayTypeInfo;
pub(crate) use traits::{GodotFfiVariant, GodotNullableFfi};

use crate::builtin::*;
use crate::global::{MethodFlags, PropertyHint, PropertyUsageFlags};
use crate::registry::method::MethodParamOrReturnInfo;
use crate::registry::property::{Export, PropertyHintInfo, Var};
use godot_ffi as sys;

#[doc(hidden)]
pub use signature::*;

#[cfg(feature = "trace")]
pub use signature::trace;

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Describes a property in Godot.
///
/// Abstraction of the low-level `sys::GDExtensionPropertyInfo`.
///
/// Keeps the actual allocated values (the `sys` equivalent only keeps pointers, which fall out of scope).
#[derive(Debug, Clone)]
// Note: is not #[non_exhaustive], so adding fields is a breaking change. Mostly used internally at the moment though.
pub struct PropertyInfo {
    /// Which type this property has.
    ///
    /// For objects this should be set to [`VariantType::OBJECT`], and the `class_name` field to the actual name of the class.
    ///
    /// For [`Variant`] this should be set to [`VariantType::NIL`].
    pub variant_type: VariantType,

    /// Which class this property is.
    ///
    /// This should be set to [`ClassName::none()`] unless the variant type is `Object`. You can use
    /// [`GodotClass::class_name()`](crate::obj::GodotClass::class_name()) to get the right name to use here.
    pub class_name: ClassName,

    /// The name of this property in Godot.
    pub property_name: StringName,

    /// How the property is meant to be edited. See also [`PropertyHint`] in the Godot docs.
    ///
    /// [`PropertyHint`]: https://docs.godotengine.org/en/latest/classes/class_%40globalscope.html#enum-globalscope-propertyhint
    pub hint: PropertyHint,

    /// Extra information passed to Godot for this property, what this means depends on the `hint` value.
    pub hint_string: GString,

    /// How this property should be used. See [`PropertyUsageFlags`] in Godot for the meaning.
    ///
    /// [`PropertyUsageFlags`]: https://docs.godotengine.org/en/latest/classes/class_%40globalscope.html#enum-globalscope-propertyusageflags
    pub usage: PropertyUsageFlags,
}

impl PropertyInfo {
    /// Create a new `PropertyInfo` representing a property named `property_name` with type `T`.
    ///
    /// This will generate property info equivalent to what a `#[var]` attribute would.
    pub fn new_var<T: Var>(property_name: &str) -> Self {
        <T as GodotConvert>::Via::property_info(property_name).with_hint_info(T::property_hint())
    }

    /// Create a new `PropertyInfo` representing an exported property named `property_name` with type `T`.
    ///
    /// This will generate property info equivalent to what an `#[export]` attribute would.
    pub fn new_export<T: Export>(property_name: &str) -> Self {
        <T as GodotConvert>::Via::property_info(property_name)
            .with_hint_info(T::default_export_info())
    }

    /// Change the `hint` and `hint_string` to be the given `hint_info`.
    ///
    /// See [`export_info_functions`](crate::registry::property::export_info_functions) for functions that return appropriate `PropertyHintInfo`s for
    /// various Godot annotations.
    ///
    /// # Examples
    ///
    /// Creating an `@export_range` property.
    ///
    // TODO: Make this nicer to use.
    /// ```no_run
    /// use godot::register::property::export_info_functions;
    /// use godot::meta::PropertyInfo;
    ///
    /// let property = PropertyInfo::new_export::<f64>("my_range_property")
    ///     .with_hint_info(export_info_functions::export_range(
    ///         0.0,
    ///         10.0,
    ///         Some(0.1),
    ///         false,
    ///         false,
    ///         false,
    ///         false,
    ///         false,
    ///         false,
    ///         Some("mm".to_string()),
    ///     ));
    /// ```
    pub fn with_hint_info(self, hint_info: PropertyHintInfo) -> Self {
        let PropertyHintInfo { hint, hint_string } = hint_info;

        Self {
            hint,
            hint_string,
            ..self
        }
    }

    /// Create a new `PropertyInfo` representing a group in Godot.
    ///
    /// See [`EditorInspector`](https://docs.godotengine.org/en/latest/classes/class_editorinspector.html#class-editorinspector) in Godot for
    /// more information.
    pub fn new_group(group_name: &str, group_prefix: &str) -> Self {
        Self {
            variant_type: VariantType::NIL,
            class_name: ClassName::none(),
            property_name: group_name.into(),
            hint: PropertyHint::NONE,
            hint_string: group_prefix.into(),
            usage: PropertyUsageFlags::GROUP,
        }
    }

    /// Create a new `PropertyInfo` representing a subgroup in Godot.
    ///
    /// See [`EditorInspector`](https://docs.godotengine.org/en/latest/classes/class_editorinspector.html#class-editorinspector) in Godot for
    /// more information.
    pub fn new_subgroup(subgroup_name: &str, subgroup_prefix: &str) -> Self {
        Self {
            variant_type: VariantType::NIL,
            class_name: ClassName::none(),
            property_name: subgroup_name.into(),
            hint: PropertyHint::NONE,
            hint_string: subgroup_prefix.into(),
            usage: PropertyUsageFlags::SUBGROUP,
        }
    }

    /// Converts to the FFI type. Keep this object allocated while using that!
    pub fn property_sys(&self) -> sys::GDExtensionPropertyInfo {
        use crate::obj::EngineBitfield as _;
        use crate::obj::EngineEnum as _;

        sys::GDExtensionPropertyInfo {
            type_: self.variant_type.sys(),
            name: sys::SysPtr::force_mut(self.property_name.string_sys()),
            class_name: sys::SysPtr::force_mut(self.class_name.string_sys()),
            hint: u32::try_from(self.hint.ord()).expect("hint.ord()"),
            hint_string: sys::SysPtr::force_mut(self.hint_string.string_sys()),
            usage: u32::try_from(self.usage.ord()).expect("usage.ord()"),
        }
    }

    pub fn empty_sys() -> sys::GDExtensionPropertyInfo {
        use crate::obj::EngineBitfield as _;
        use crate::obj::EngineEnum as _;

        sys::GDExtensionPropertyInfo {
            type_: VariantType::NIL.sys(),
            name: std::ptr::null_mut(),
            class_name: std::ptr::null_mut(),
            hint: PropertyHint::NONE.ord() as u32,
            hint_string: std::ptr::null_mut(),
            usage: PropertyUsageFlags::NONE.ord() as u32,
        }
    }

    /// Consumes self and turns it into a `sys::GDExtensionPropertyInfo`, should be used together with
    /// [`free_owned_property_sys`](Self::free_owned_property_sys).
    ///
    /// This will leak memory unless used together with `free_owned_property_sys`.
    pub(crate) fn into_owned_property_sys(self) -> sys::GDExtensionPropertyInfo {
        use crate::obj::EngineBitfield as _;
        use crate::obj::EngineEnum as _;

        sys::GDExtensionPropertyInfo {
            type_: self.variant_type.sys(),
            name: self.property_name.into_owned_string_sys(),
            class_name: sys::SysPtr::force_mut(self.class_name.string_sys()),
            hint: u32::try_from(self.hint.ord()).expect("hint.ord()"),
            hint_string: self.hint_string.into_owned_string_sys(),
            usage: u32::try_from(self.usage.ord()).expect("usage.ord()"),
        }
    }

    /// Properly frees a `sys::GDExtensionPropertyInfo` created by [`into_owned_property_sys`](Self::into_owned_property_sys).
    ///
    /// # Safety
    ///
    /// * Must only be used on a struct returned from a call to `into_owned_property_sys`, without modification.
    /// * Must not be called more than once on a `sys::GDExtensionPropertyInfo` struct.
    pub(crate) unsafe fn free_owned_property_sys(info: sys::GDExtensionPropertyInfo) {
        // SAFETY: This function was called on a pointer returned from `into_owned_property_sys`, thus both `info.name` and
        // `info.hint_string` were created from calls to `into_owned_string_sys` on their respective types.
        // Additionally, this function isn't called more than once on a struct containing the same `name` or `hint_string` pointers.
        unsafe {
            let _name = StringName::from_owned_string_sys(info.name);
            let _hint_string = GString::from_owned_string_sys(info.hint_string);
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Describes a method in Godot.
///
/// Abstraction of the low-level `sys::GDExtensionMethodInfo`.
// Currently used for ScriptInstance.
// TODO check overlap with (private) ClassMethodInfo.
#[derive(Debug, Clone)]
pub struct MethodInfo {
    pub id: i32,
    pub method_name: StringName,
    pub class_name: ClassName,
    pub return_type: PropertyInfo,
    pub arguments: Vec<PropertyInfo>,
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

/// Clean up various resources at end of usage.
///
/// # Safety
/// Must not use meta facilities (e.g. `ClassName`) after this call.
pub(crate) unsafe fn cleanup() {
    class_name::cleanup();
}
