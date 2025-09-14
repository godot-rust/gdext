/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot_ffi as sys;

use crate::builtin;
use crate::builtin::{Variant, VariantType};
use crate::global::PropertyUsageFlags;
use crate::meta::error::ConvertError;
use crate::meta::{
    sealed, ClassName, FromGodot, GodotConvert, PropertyHintInfo, PropertyInfo, ToGodot,
};
use crate::registry::method::MethodParamOrReturnInfo;
use crate::registry::property::builtin_type_string;

// Re-export sys traits in this module, so all are in one place.
#[rustfmt::skip] // Do not reorder.
pub use sys::{ExtVariantType, GodotFfi, GodotNullableFfi};

pub use crate::builtin::meta_reexport::PackedArrayElement;

/// Conversion of [`GodotFfi`] types to/from [`Variant`].
#[doc(hidden)]
pub trait GodotFfiVariant: Sized + GodotFfi {
    fn ffi_to_variant(&self) -> Variant;
    fn ffi_from_variant(variant: &Variant) -> Result<Self, ConvertError>;
}

/// Type that is directly representable in the engine.
///
/// This trait cannot be implemented for custom user types; for those, [`GodotConvert`] exists instead.
/// A type implements `GodotType` when Godot has a direct, native representation for it. For instance:
/// - [`i64`] implements `GodotType`, since it can be directly represented by Godot's `int` type.
/// - But [`VariantType`][crate::builtin::VariantType] does not implement `GodotType`. While it is an enum Godot uses,
///   we have no native way to indicate to Godot that a value should be one of the variants of `VariantType`.
//
// Unlike `GodotFfi`, types implementing this trait don't need to fully represent its corresponding Godot
// type. For instance [`i32`] does not implement `GodotFfi` because it cannot represent all values of
// Godot's `int` type, however it does implement `GodotType` because we can set the meta-data of values with
// this type to indicate that they are 32 bits large.
pub trait GodotType: GodotConvert<Via = Self> + sealed::Sealed + Sized + 'static
// 'static is not technically required, but it simplifies a few things (limits e.g. `ObjectArg`).
{
    // Value type for this type's FFI representation.
    #[doc(hidden)]
    type Ffi: GodotFfiVariant + 'static;

    // Value or reference type when passing this type *to* Godot FFI.
    #[doc(hidden)]
    type ToFfi<'f>: GodotFfiVariant
    where
        Self: 'f;

    /// Returns the FFI representation of this type, used for argument passing.
    ///
    /// Often returns a reference to the value, which can then be used to interact with Godot without cloning/inc-ref-ing the value.
    /// For scalars and `Copy` types, this usually returns a copy of the value.
    #[doc(hidden)]
    fn to_ffi(&self) -> Self::ToFfi<'_>;

    /// Consumes value and converts into FFI representation, used for return types.
    ///
    /// Unlike [`to_ffi()`][Self:to_ffi], this method consumes the value and is used for return types rather than argument passing.
    /// Using `to_ffi()` for return types can be incorrect, since the associated types `Ffi` and `ToFfi<'f>` may differ and the latter
    /// may not implement return type conversions such as [`GodotFfi::move_return_ptr()`].
    #[doc(hidden)]
    fn into_ffi(self) -> Self::Ffi;

    /// Converts from FFI representation to Rust type.
    #[doc(hidden)]
    fn try_from_ffi(ffi: Self::Ffi) -> Result<Self, ConvertError>;

    #[doc(hidden)]
    fn from_ffi(ffi: Self::Ffi) -> Self {
        Self::try_from_ffi(ffi).expect("Failed conversion from FFI representation to Rust type")
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
            variant_type: Self::Ffi::VARIANT_TYPE.variant_as_nil(),
            class_name: Self::class_name(),
            property_name: builtin::StringName::from(property_name),
            hint_info: Self::property_hint_info(),
            usage: PropertyUsageFlags::DEFAULT,
        }
    }

    #[doc(hidden)]
    fn property_hint_info() -> PropertyHintInfo {
        // The default implementation is mostly good for builtin types.
        //PropertyHintInfo::with_type_name::<Self>()

        PropertyHintInfo::none()
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

    /// Returns a string representation of the Godot type name, as it is used in several property hint contexts.
    ///
    /// Examples:
    /// - `MyClass` for objects
    /// - `StringName`, `AABB` or `int` for built-ins
    /// - `Array` for arrays
    #[doc(hidden)]
    fn godot_type_name() -> String;

    /// Special-casing for `FromVariant` conversions higher up: true if the variant can be interpreted as `Option<Self>::None`.
    ///
    /// Returning false only means that this is not a special case, not that it cannot be `None`. Regular checks are expected to run afterward.
    ///
    /// This exists only for var-calls and serves a similar purpose as `GodotNullableFfi::is_null()` (although that handles general cases).
    #[doc(hidden)]
    fn qualifies_as_special_none(_from_variant: &Variant) -> bool {
        false
    }

    /// Convert to `ObjectArg` for efficient object argument passing.
    ///
    /// Implemented in `GodotType` because Rust has no specialization, and there's no good way to have trait bounds in `ByObject`, but not in
    /// other arg-passing strategies `ByValue`/`ByRef`.
    ///
    /// # Panics
    /// If `Self` is not an object type (`Gd<T>`, `Option<Gd<T>>`). Note that `DynGd<T>` isn't directly implemented here, but uses `Gd<T>`'s
    /// impl on the FFI layer.
    ///
    /// # Safety
    /// `self` must be kept alive while return value is in use. Might be addressed with lifetime in the future.
    #[doc(hidden)]
    unsafe fn as_object_arg(&self) -> crate::meta::ObjectArg {
        panic!(
            "as_object_arg() called for non-object type: {}",
            std::any::type_name::<Self>()
        )
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Marker trait to identify types that can be stored in [`Array<T>`][crate::builtin::Array].
///
/// The types, for which this trait is implemented, overlap mostly with [`GodotType`].
///
/// Notable differences are:
/// - Only `VariantArray`, not `Array<T>` is allowed (typed arrays cannot be nested).
/// - `Option` is only supported for `Option<Gd<T>>`, but not e.g. `Option<i32>`.
///
/// # Integer and float types
/// `u8`, `i8`, `u16`, `i16`, `u32`, `i32` and `f32` are supported by this trait, however they don't have their own array type in Godot.
/// The engine only knows about `i64` ("int") and `f64` ("float") types. This means that when using any integer or float type, Godot
/// will treat it as the equivalent of GDScript's `Array[int]` or `Array[float]`, respectively.
///
/// As a result, when converting from a Godot typed array to a Rust `Array<T>`, the values stored may not actually fit into a `T`.
/// For example, you have a GDScript `Array[int]` which stores value 160, and you convert it to a Rust `Array<i8>`. This means that you may
/// end up with panics on element access (since the `Variant` storing 160 will fail to convert to `i8`). In Debug mode, we add additional
/// best-effort checks to detect such errors, however they are expensive and not bullet-proof. If you need very rigid type safety, stick to
/// `i64` and `f64`. The other types however can be extremely convenient and work well, as long as you are aware of the limitations.
///
/// `u64` is entirely unsupported since it cannot be safely stored inside a `Variant`.
///
/// Also, keep in mind that Godot uses `Variant` for each element. If performance matters and you have small element types such as `u8`,
/// consider using packed arrays (e.g. `PackedByteArray`) instead.
//
#[diagnostic::on_unimplemented(
    message = "`Array<T>` can only store element types supported in Godot arrays (no nesting).",
    label = "has invalid element type"
)]
pub trait ArrayElement: ToGodot + FromGodot + sealed::Sealed + 'static {
    // Note: several indirections in `ArrayElement` and the global `element_*` functions go through `GodotConvert::Via`,
    // to not require Self: `GodotType`. What matters is how array elements map to Godot on the FFI level (`GodotType` trait).

    /// Returns the representation of this type as a type string, e.g. `"4:"` for string, or `"24:34/MyClass"` for objects.
    ///
    /// (`4` and `24` are variant type ords; `34` is `PropertyHint::NODE_TYPE` ord).
    ///
    /// Used for elements in arrays (the latter despite `ArrayElement` not having a direct relation).
    ///
    /// See [`PropertyHint::TYPE_STRING`] and
    /// [upstream docs](https://docs.godotengine.org/en/stable/classes/class_%40globalscope.html#enum-globalscope-propertyhint).
    #[doc(hidden)]
    fn element_type_string() -> String {
        // Most array elements and all packed array elements are builtin types, so this is a good default.
        builtin_type_string::<Self::Via>()
    }

    #[doc(hidden)]
    fn debug_validate_elements(_array: &builtin::Array<Self>) -> Result<(), ConvertError> {
        // No-op for most element types.
        Ok(())
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Non-polymorphic helper functions, to avoid constant `<T::Via as GodotType>::` in the code.

#[doc(hidden)]
pub(crate) const fn element_variant_type<T: ArrayElement>() -> VariantType {
    <T::Via as GodotType>::Ffi::VARIANT_TYPE.variant_as_nil()
}

/// Classifies `T` into one of Godot's builtin types. **Important:** variants are mapped to `NIL`.
#[doc(hidden)]
pub(crate) const fn ffi_variant_type<T: GodotConvert>() -> ExtVariantType {
    <T::Via as GodotType>::Ffi::VARIANT_TYPE
}

#[doc(hidden)]
pub(crate) fn element_godot_type_name<T: ArrayElement>() -> String {
    <T::Via as GodotType>::godot_type_name()
}

// #[doc(hidden)]
// pub(crate)  fn element_godot_type_name<T: ArrayElement>() -> String {
//     <T::Via as GodotType>::godot_type_name()
// }
