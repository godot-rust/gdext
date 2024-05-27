/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Meta-information about variant types, properties and class names.

mod array_type_info;
mod class_name;
mod godot_convert;
mod sealed;
mod signature;
mod traits;

pub mod error;
pub use class_name::ClassName;
pub use godot_convert::{FromGodot, GodotConvert, ToGodot};
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
        // Additionally this function isn't called more than once on a struct containing the same `name` or `hint_string` pointers.
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
    /// Converts to the FFI type. Keep this object allocated while using that!
    ///
    /// The struct returned by this function contains pointers into the fields of `self`. `self` should therefore not be dropped while the
    /// `sys::GDExtensionMethodInfo` is still in use.
    ///
    /// This function also leaks memory that has to be cleaned up by the caller once it is no longer used. Specifically the `arguments` and
    /// `default_arguments` vectors have to be reconstructed from the pointer and length and then dropped/freed.
    ///
    /// Each vector can be reconstructed with `Vec::from_raw_parts` since the pointers were created with `Vec::into_boxed_slice`, which
    /// guarantees that the vector capacity and length are equal.
    pub fn method_sys(&self) -> sys::GDExtensionMethodInfo {
        use crate::obj::EngineBitfield as _;

        let argument_count = self.arguments.len() as u32;
        let argument_vec = self
            .arguments
            .iter()
            .map(|arg| arg.property_sys())
            .collect::<Vec<_>>()
            .into_boxed_slice();

        // SAFETY: dereferencing the new box pointer is fine as it is guaranteed to not be null
        let arguments = unsafe { (*Box::into_raw(argument_vec)).as_mut_ptr() };

        let default_argument_count = self.default_arguments.len() as u32;
        let default_argument_vec = self
            .default_arguments
            .iter()
            .map(|arg| sys::SysPtr::force_mut(arg.var_sys()))
            .collect::<Vec<_>>()
            .into_boxed_slice();

        // SAFETY: dereferencing the new box pointer is fine as it is guaranteed to not be null
        let default_arguments = unsafe { (*Box::into_raw(default_argument_vec)).as_mut_ptr() };

        sys::GDExtensionMethodInfo {
            id: self.id,
            name: sys::SysPtr::force_mut(self.method_name.string_sys()),
            return_value: self.return_type.property_sys(),
            argument_count,
            arguments,
            default_argument_count,
            default_arguments,
            flags: u32::try_from(self.flags.ord()).expect("flags should be valid"),
        }
    }
}
