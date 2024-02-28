/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

pub mod registration;

mod class_name;
mod godot_convert;
mod return_marshal;
mod signature;

pub use class_name::*;
pub use godot_convert::*;
#[doc(hidden)]
pub use return_marshal::*;
#[doc(hidden)]
pub use signature::*;

pub(crate) use godot_convert::convert_error::*;

use crate::builtin::*;
use crate::engine::global;
use crate::property::{Export, PropertyHintInfo, Var};
use godot_ffi as sys;
use registration::method::MethodParamOrReturnInfo;
use sys::{GodotFfi, GodotNullableFfi};

/// Conversion of [`GodotFfi`] types to/from [`Variant`].
#[doc(hidden)]
pub trait GodotFfiVariant: Sized + GodotFfi {
    fn ffi_to_variant(&self) -> Variant;
    fn ffi_from_variant(variant: &Variant) -> Result<Self, ConvertError>;
}

mod sealed {
    // To ensure the user does not implement `GodotType` for their own types.

    use godot_ffi::GodotNullableFfi;

    use super::GodotType;
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
    impl<T: GodotType> Sealed for Array<T> {}
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
            hint: global::PropertyHint::NONE,
            hint_string: GString::new(),
            usage: global::PropertyUsageFlags::DEFAULT,
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

/// Representation of a property in Godot.
///
/// Stores all info needed to inform Godot about how to interpret a property. This is used for actual properties, function arguments,
/// return types, signal arguments, and other similar cases.
///
/// A mismatch between property info and the actual type of a property may lead to runtime errors as Godot tries to use the property in the
/// wrong way, such as by inserting the wrong type or expecting a different type to be returned.
///
/// Rusty abstraction of `sys::GDExtensionPropertyInfo`, keeps the actual allocated values (the `sys` equivalent only keeps pointers, which
/// fall out of scope).
#[derive(Debug)]
// It is uncertain if we want to add more fields to this in the future, so we'll mark it `non_exhaustive` as a precautionary measure.
#[non_exhaustive]
pub struct PropertyInfo {
    /// The variant type of the property.
    ///
    /// Note that for classes this will be `Object`, and the `class_name` field will specify what specific class this property is.
    pub variant_type: VariantType,
    /// The class name of the property.
    ///
    /// This only matters if `variant_type` is `Object`. Otherwise it's ignored by Godot.
    pub class_name: ClassName,
    /// The name this property will have in Godot.
    pub property_name: StringName,
    /// The property hint that will determine how Godot interprets this value.
    ///
    /// See Godot docs for more information:
    /// * [`PropertyHint`](https://docs.godotengine.org/en/stable/classes/class_%40globalscope.html#enum-globalscope-propertyhint).
    pub hint: global::PropertyHint,
    /// Extra information used in conjunction with `hint`.
    pub hint_string: GString,
    /// How Godot will use this property.
    ///
    /// See Godot docs for more inormation:
    /// * [`PropertyUsageFlags`](https://docs.godotengine.org/en/stable/classes/class_%40globalscope.html#enum-globalscope-propertyusageflags).
    pub usage: global::PropertyUsageFlags,
}

impl PropertyInfo {
    /// Create a new `PropertyInfo` for a property that isn't exported to the editor.
    ///
    /// `P` is the type the property will be declared as, and `property_name` is the name the property will have.  
    pub fn new_var<P: Var>(property_name: &str) -> Self {
        let PropertyHintInfo { hint, hint_string } = P::property_hint();

        Self {
            hint,
            hint_string,
            usage: global::PropertyUsageFlags::NO_EDITOR,
            ..P::Via::property_info(property_name)
        }
    }

    /// Create a new `PropertyInfo` for a property that is exported to the editor.
    ///
    /// `P` is the type the property will be declared as, and `property_name` is the name the property will have.  
    pub fn new_export<P: Export>(property_name: &str) -> Self {
        let PropertyHintInfo { hint, hint_string } = P::default_export_info();

        Self {
            hint,
            hint_string,
            usage: global::PropertyUsageFlags::DEFAULT,
            ..P::Via::property_info(property_name)
        }
    }

    /// Create a new `PropertyInfo` for the return type of a method.
    ///
    /// `P` is the type the property will be declared as.  
    pub fn new_return<P: ToGodot>() -> Self {
        Self {
            usage: global::PropertyUsageFlags::NONE,
            ..P::Via::property_info("")
        }
    }

    /// Create a new `PropertyInfo` for an argument of a method.
    ///
    /// `P` is the type the property will be declared as, and `property_name` is the name the argument will have.  
    pub fn new_arg<P: FromGodot>(arg_name: &str) -> Self {
        Self {
            usage: global::PropertyUsageFlags::NONE,
            ..P::Via::property_info(arg_name)
        }
    }

    /// Converts to the FFI type. Keep this object allocated while using that!
    pub fn property_sys(&self) -> sys::GDExtensionPropertyInfo {
        use crate::obj::EngineBitfield as _;
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

    /// Converts to the FFI type.
    ///
    /// Unlike [`property_sys`](PropertyInfo::property_sys) this object does not need to be kept allocated while using the returned value,
    /// however if you do not explicitly free the returned value at some point then this will lead to a memory leak. See
    /// [`drop_property_sys`](PropertyInfo::drop_property_sys).
    pub fn into_property_sys(self) -> sys::GDExtensionPropertyInfo {
        use crate::obj::EngineBitfield as _;
        use crate::obj::EngineEnum as _;

        let Self {
            variant_type,
            class_name,
            property_name,
            hint,
            hint_string,
            usage,
        } = self;

        sys::GDExtensionPropertyInfo {
            type_: variant_type.sys(),
            name: property_name.into_string_sys(),
            class_name: class_name.string_sys(),
            hint: u32::try_from(hint.ord()).expect("hint.ord()"),
            hint_string: hint_string.into_string_sys(),
            usage: u32::try_from(usage.ord()).expect("usage.ord()"),
        }
    }

    /// Consumes a [sys::GDExtensionPropertyInfo].
    ///
    /// # Safety
    ///
    /// The given property info must have been returned from a call to [`into_property_sys`](PropertyInfo::into_property_sys).
    pub unsafe fn drop_property_sys(property_sys: sys::GDExtensionPropertyInfo) {
        let _property_name = StringName::from_string_sys(property_sys.name);
        let _hint_string = GString::from_string_sys(property_sys.hint_string);
    }

    pub fn empty_sys() -> sys::GDExtensionPropertyInfo {
        use crate::obj::EngineBitfield as _;
        use crate::obj::EngineEnum as _;

        sys::GDExtensionPropertyInfo {
            type_: VariantType::Nil.sys(),
            name: std::ptr::null_mut(),
            class_name: std::ptr::null_mut(),
            hint: global::PropertyHint::NONE.ord() as u32,
            hint_string: std::ptr::null_mut(),
            usage: global::PropertyUsageFlags::NONE.ord() as u32,
        }
    }
}

#[derive(Debug)]
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
            .map(|arg| arg.var_sys())
            .collect::<Vec<_>>()
            .into_boxed_slice();

        // SAFETY: dereferencing the new box pointer is fine as it is guaranteed to not be null
        let default_arguments = unsafe { (*Box::into_raw(default_argument_vec)).as_mut_ptr() };

        sys::GDExtensionMethodInfo {
            id: self.id,
            name: self.method_name.string_sys(),
            return_value: self.return_type.property_sys(),
            argument_count,
            arguments,
            default_argument_count,
            default_arguments,
            flags: u32::try_from(self.flags.ord()).expect("flags should be valid"),
        }
    }
}
