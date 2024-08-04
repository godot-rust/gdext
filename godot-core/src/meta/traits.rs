/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot_ffi as sys;

use crate::builtin::{StringName, Variant};
use crate::global::PropertyUsageFlags;
use crate::meta::error::ConvertError;
use crate::meta::{
    sealed, ClassName, FromGodot, GodotConvert, PropertyHintInfo, PropertyInfo, ToGodot,
};
use crate::registry::method::MethodParamOrReturnInfo;

// Re-export sys traits in this module, so all are in one place.
use crate::registry::property::builtin_type_string;
pub use sys::{GodotFfi, GodotNullableFfi};

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
// Godot's `int` type, however it does implement `GodotType` because we can set the metadata of values with
// this type to indicate that they are 32 bits large.
pub trait GodotType:
    GodotConvert<Via = Self> + ToGodot + FromGodot + sealed::Sealed + 'static
// 'static is not technically required, but it simplifies a few things (limits e.g. ObjectArg).
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

    #[doc(hidden)]
    fn godot_type_name() -> String;

    /// Special-casing for `FromVariant` conversions higher up: true if the variant can be interpreted as `Option<Self>::None`.
    ///
    /// Returning false only means that this is not a special case, not that it cannot be `None`. Regular checks are expected to run afterward.
    ///
    /// This exists only for varcalls and serves a similar purpose as `GodotNullableFfi::is_null()` (although that handles general cases).
    #[doc(hidden)]
    fn qualifies_as_special_none(_from_variant: &Variant) -> bool {
        false
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
pub trait ArrayElement: GodotType + sealed::Sealed {
    /// Returns the representation of this type as a type string.
    ///
    /// Used for elements in arrays and packed arrays (the latter despite `ArrayElement` not having a direct relation).
    ///
    /// See [`PropertyHint::TYPE_STRING`] and [upstream docs].
    ///
    /// [upstream docs]: https://docs.godotengine.org/en/stable/classes/class_%40globalscope.html#enum-globalscope-propertyhint
    #[doc(hidden)]
    fn element_type_string() -> String {
        // Most array elements and all packed array elements are builtin types, so this is a good default.
        builtin_type_string::<Self>()
    }
}
