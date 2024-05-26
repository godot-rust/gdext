/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

pub mod registration;

mod call_error;
mod class_name;
mod godot_convert;
mod signature;

pub use call_error::*;
pub use class_name::*;
pub use godot_convert::*;
#[doc(hidden)]
pub use signature::*;

pub(crate) use godot_convert::convert_error::*;

use crate::builtin::*;
use crate::global::{self, PropertyHint, PropertyUsageFlags};
use crate::property::Var;
use crate::property::{Export, PropertyHintInfo};
use godot_ffi as sys;
use registration::method::MethodParamOrReturnInfo;
use sys::{GodotFfi, GodotNullableFfi};

#[cfg(feature = "trace")]
pub use signature::trace;

/// Conversion of [`GodotFfi`] types to/from [`Variant`].
#[doc(hidden)]
pub trait GodotFfiVariant: Sized + GodotFfi {
    fn ffi_to_variant(&self) -> Variant;
    fn ffi_from_variant(variant: &Variant) -> Result<Self, ConvertError>;
}

mod sealed {
    // To ensure the user does not implement `GodotType` for their own types.

    use godot_ffi::GodotNullableFfi;

    use super::{ArrayElement, GodotType};
    use crate::builtin::*;
    use crate::obj::*;

    pub trait Sealed {}

    impl Sealed for Aabb {}
    impl Sealed for Basis {}
    impl Sealed for Callable {}
    impl Sealed for Vector2 {}
    impl Sealed for Vector3 {}
    impl Sealed for Vector4 {}
    impl Sealed for Vector2i {}
    impl Sealed for Vector3i {}
    impl Sealed for Vector4i {}
    impl Sealed for Quaternion {}
    impl Sealed for Color {}
    impl Sealed for GString {}
    impl Sealed for StringName {}
    impl Sealed for NodePath {}
    impl Sealed for PackedByteArray {}
    impl Sealed for PackedInt32Array {}
    impl Sealed for PackedInt64Array {}
    impl Sealed for PackedFloat32Array {}
    impl Sealed for PackedFloat64Array {}
    impl Sealed for PackedStringArray {}
    impl Sealed for PackedVector2Array {}
    impl Sealed for PackedVector3Array {}
    #[cfg(since_api = "4.3")]
    impl Sealed for PackedVector4Array {}
    impl Sealed for PackedColorArray {}
    impl Sealed for Plane {}
    impl Sealed for Projection {}
    impl Sealed for Rid {}
    impl Sealed for Rect2 {}
    impl Sealed for Rect2i {}
    impl Sealed for Signal {}
    impl Sealed for Transform2D {}
    impl Sealed for Transform3D {}
    impl Sealed for Dictionary {}
    impl Sealed for bool {}
    impl Sealed for i64 {}
    impl Sealed for i32 {}
    impl Sealed for i16 {}
    impl Sealed for i8 {}
    impl Sealed for u64 {}
    impl Sealed for u32 {}
    impl Sealed for u16 {}
    impl Sealed for u8 {}
    impl Sealed for f64 {}
    impl Sealed for f32 {}
    impl Sealed for () {}
    impl Sealed for Variant {}
    impl<T: ArrayElement> Sealed for Array<T> {}
    impl<T: GodotClass> Sealed for RawGd<T> {}
    impl<T: GodotClass> Sealed for Gd<T> {}
    impl<T> Sealed for Option<T>
    where
        T: GodotType,
        T::Ffi: GodotNullableFfi,
    {
    }
}

/// Type that is directly representable in the engine.
///
/// This trait cannot be implemented for custom user types; for those, [`GodotConvert`] exists instead.
/// A type implements `GodotType` when Godot has a direct, native representation for it. For instance:
/// - [`i64`] implements `GodotType`, since it can be directly represented by Godot's `int` type.
/// - But [`VariantType`] does not implement `GodotType`. While it is an enum Godot uses, we have no native way to indicate
///   to Godot that a value should be one of the variants of `VariantType`.
//
// Unlike `GodotFfi`, types implementing this trait don't need to fully represent its corresponding Godot
// type. For instance [`i32`] does not implement `GodotFfi` because it cannot represent all values of
// Godot's `int` type, however it does implement `GodotType` because we can set the metadata of values with
// this type to indicate that they are 32 bits large.
pub trait GodotType:
    GodotConvert<Via = Self> + ToGodot + FromGodot + sealed::Sealed + 'static
{
    #[doc(hidden)]
    type Ffi: GodotFfiVariant;

    #[doc(hidden)]
    fn to_ffi(&self) -> Self::Ffi;

    #[doc(hidden)]
    fn into_ffi(self) -> Self::Ffi;

    #[doc(hidden)]
    fn try_from_ffi(ffi: Self::Ffi) -> Result<Self, ConvertError>;

    #[doc(hidden)]
    fn from_ffi(ffi: Self::Ffi) -> Self {
        Self::try_from_ffi(ffi).unwrap()
    }

    #[doc(hidden)]
    fn param_metadata() -> sys::GDExtensionClassMethodArgumentMetadata {
        Self::Ffi::default_param_metadata()
    }

    #[doc(hidden)]
    fn class_name() -> ClassName {
        // If we use `ClassName::of::<()>()` then this type shows up as `(no base)` in documentation.
        ClassName::none()
    }

    #[doc(hidden)]
    fn property_info(property_name: &str) -> PropertyInfo {
        PropertyInfo {
            variant_type: Self::Ffi::variant_type(),
            class_name: Self::class_name(),
            property_name: StringName::from(property_name),
            hint: PropertyHint::NONE,
            hint_string: GString::new(),
            usage: PropertyUsageFlags::DEFAULT,
        }
    }

    #[doc(hidden)]
    fn argument_info(property_name: &str) -> MethodParamOrReturnInfo {
        MethodParamOrReturnInfo::new(Self::property_info(property_name), Self::param_metadata())
    }

    #[doc(hidden)]
    fn return_info() -> Option<MethodParamOrReturnInfo> {
        Some(MethodParamOrReturnInfo::new(
            Self::property_info(""),
            Self::param_metadata(),
        ))
    }

    #[doc(hidden)]
    fn godot_type_name() -> String;

    /// Special-casing for `FromVariant` conversions higher up: true if the variant can be interpreted as `Option<Self>::None`.
    ///
    /// Returning false only means that this is not a special case, not that it cannot be `None`. Regular checks are expected to run afterwards.
    ///
    /// This exists only for varcalls and serves a similar purpose as `GodotNullableFfi::is_null()` (although that handles general cases).
    #[doc(hidden)]
    fn qualifies_as_special_none(_from_variant: &Variant) -> bool {
        false
    }
}

impl<T> GodotType for Option<T>
where
    T: GodotType,
    T::Ffi: GodotNullableFfi,
{
    type Ffi = T::Ffi;

    fn to_ffi(&self) -> Self::Ffi {
        GodotNullableFfi::flatten_option(self.as_ref().map(|t| t.to_ffi()))
    }

    fn into_ffi(self) -> Self::Ffi {
        GodotNullableFfi::flatten_option(self.map(|t| t.into_ffi()))
    }

    fn try_from_ffi(ffi: Self::Ffi) -> Result<Self, ConvertError> {
        if ffi.is_null() {
            return Ok(None);
        }

        GodotType::try_from_ffi(ffi).map(Some)
    }

    fn from_ffi(ffi: Self::Ffi) -> Self {
        if ffi.is_null() {
            return None;
        }

        Some(GodotType::from_ffi(ffi))
    }

    fn param_metadata() -> sys::GDExtensionClassMethodArgumentMetadata {
        T::param_metadata()
    }

    fn class_name() -> ClassName {
        T::class_name()
    }

    fn property_info(property_name: &str) -> PropertyInfo {
        T::property_info(property_name)
    }

    fn argument_info(property_name: &str) -> MethodParamOrReturnInfo {
        T::argument_info(property_name)
    }

    fn return_info() -> Option<MethodParamOrReturnInfo> {
        T::return_info()
    }

    fn godot_type_name() -> String {
        T::godot_type_name()
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Marker trait to identify types that can be stored in [`Array<T>`][crate::builtin::Array].
///
/// The types for which this trait is implemented, overlap mostly with [`GodotType`].
/// This is done consistently what GDScript allows inside `Array[T]`.
///
/// Notable differences are:
/// - Only `VariantArray`, not `Array<T>` is allowed (typed arrays cannot be nested).
/// - `Option` is only supported for `Option<Gd<T>>`, but not e.g. `Option<i32>`.
#[diagnostic::on_unimplemented(
    message = "`Array<T>` can only store element types supported in Godot arrays (no nesting).",
    label = "does not implement `Var`",
    note = "see also: https://godot-rust.github.io/docs/gdext/master/godot/builtin/meta/trait.ArrayElement.html"
)]
pub trait ArrayElement: GodotType {}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Rusty abstraction of `sys::GDExtensionPropertyInfo`.
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
    /// See [`export_info_functions`](crate::property::export_info_functions) for functions that return appropriate `PropertyHintInfo`s for
    /// various Godot annotations.
    ///
    /// # Examples
    ///
    /// Creating an `@export_range` property.
    ///
    // TODO: Make this nicer to use.
    /// ```no_run
    /// use godot::register::property::export_info_functions;
    /// use godot::builtin::meta::PropertyInfo;
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

// TODO(bromeon): Rename to ScriptMethodInfo; check difference with with existing MethodInfo; public fields good as-is?
#[derive(Debug, Clone)]
pub struct MethodInfo {
    pub id: i32,
    pub method_name: StringName,
    pub class_name: ClassName,
    pub return_type: PropertyInfo,
    pub arguments: Vec<PropertyInfo>,
    pub default_arguments: Vec<Variant>,
    pub flags: global::MethodFlags,
}

impl MethodInfo {
    /// Converts to the FFI type. Keep this object allocated while using that!
    ///
    /// The struct returned by this function contains pointers into the fields of `self`. `self` should therefore not be dropped while the
    /// [`sys::GDExtensionMethodInfo`] is still in use.
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
