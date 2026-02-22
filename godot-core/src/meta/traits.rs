/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot_ffi as sys;

use crate::builtin;
use crate::builtin::{Variant, VariantType};
use crate::meta::error::ConvertError;
use crate::meta::{FromGodot, GodotConvert, PropertyInfo, ToGodot, sealed};
use crate::registry::method::MethodParamOrReturnInfo;

// Re-export sys traits in this module, so all are in one place.
#[rustfmt::skip] // Do not reorder.
pub use sys::{ExtVariantType, GodotFfi, GodotNullableFfi};

pub use crate::builtin::meta_reexport::PackedElement;

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
    fn property_info(property_name: &str) -> PropertyInfo {
        // Used for method parameter/return type registration, not property registration. Keeps DEFAULT usage.
        Self::godot_shape().to_method_signature_property(property_name)
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
    #[doc(hidden)]
    fn as_object_arg(&self) -> crate::meta::ObjectArg<'_> {
        panic!(
            "as_object_arg() called for non-object type: {}",
            std::any::type_name::<Self>()
        )
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Marker trait to identify types that can be stored in [`Array<T>`][crate::builtin::Array] and [`Dictionary<K, V>`][crate::builtin::Dictionary].
///
/// Implemented for most types that can interact with Godot. A notable exception is `Array<T>` and `Dictionary<K, V>` -- Godot doesn't support
/// typed collections to be nested. You can still _store_ typed collections, but you need to use [`AnyArray`][crate::builtin::AnyArray] and
/// [`AnyDictionary`][crate::builtin::AnyDictionary], which can be **either** typed **or** untyped. We also don't support `VarArray` and
/// `VarDictionary` (special case of the former with `T=Variant`), because godot-rust cannot statically guarantee that the nested collections
/// are indeed untyped. In a GDScript `Array[Array]`, you can store both typed and untyped arrays, even within the same collection.
///
/// See also [`ElementType`][crate::meta::ElementType] for a runtime representation of this.
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
/// `u64` is [entirely unsupported](trait.GodotConvert.html#u64).
///
/// Also, keep in mind that Godot uses `Variant` for each element. If performance matters and you have small element types such as `u8`,
/// consider using packed arrays (e.g. `PackedByteArray`) instead.
//
// Note: `Element` does not require `Sealed`. This is intentional: user-defined enums (`#[derive(GodotConvert)]`) implement `Element`
// via generated code, so the trait must be open. Correctness is ensured by requiring `ToGodot + FromGodot` (both sealed), which
// guarantees that only types with valid Godot conversions can implement `Element`.
#[diagnostic::on_unimplemented(
    message = "Element type not supported in Godot Array or Dictionary (no nesting).",
    label = "has invalid element type"
)]
// TODO(v0.6): consider supertraits like PartialEq or Debug. For enums, align with #[derive(GodotConvert)].
pub trait Element: ToGodot + FromGodot + 'static {
    // Note: several indirections in `Element` and the global `element_*` functions go through `GodotConvert::Via`,
    // to not require Self: `GodotType`. What matters is how array elements map to Godot on the FFI level (`GodotType` trait).

    #[doc(hidden)]
    fn debug_validate_elements(_array: &builtin::Array<Self>) -> Result<(), ConvertError> {
        // No-op for most element types.
        Ok(())
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Non-polymorphic helper functions, to avoid constant `<T::Via as GodotType>::` in the code.

#[doc(hidden)]
pub const fn element_variant_type<T: Element>() -> VariantType {
    <T::Via as GodotType>::Ffi::VARIANT_TYPE.variant_as_nil()
}

/// Classifies `T` into one of Godot's builtin types. **Important:** variants are mapped to `NIL`.
#[doc(hidden)]
pub(crate) const fn ffi_variant_type<T: GodotConvert + ?Sized>() -> ExtVariantType {
    <T::Via as GodotType>::Ffi::VARIANT_TYPE
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Implemented for types that can be used as immutable default parameters in `#[func]` methods.
///
/// This trait ensures that default parameter values cannot be mutated by callers, preventing the Python "mutable default argument" problem
/// where a single default value is shared across multiple calls.
///
/// Post-processes the default value in some cases, e.g. makes `Array<T>` read-only via `into_read_only()`.
///
/// At the moment, this trait is conservatively implemented for types where immutability can be statically guaranteed.
/// Depending on usage, the API might be expanded in the future to allow defaults whose immutability is only determined
/// at runtime (e.g. untyped arrays/dictionaries where all element types are immutable).
///
/// # Safety
/// Allows to use the implementors in a limited `Sync` context. Implementing this trait asserts that `Self` is either:
/// - `Copy`, i.e. each instance is truly independent.
/// - Thread-safe in the sense that `clone()` is thread-safe. Individual clones must not offer a way to mutate the value or cause race conditions.
#[diagnostic::on_unimplemented(
    message = "#[opt(default = ...)] only supports a set of truly immutable types",
    label = "this type is not immutable and thus not eligible for a default value"
)]
pub unsafe trait GodotImmutable: GodotConvert + Sized {
    fn into_runtime_immutable(self) -> Self {
        self
    }
}

mod godot_immutable_impls {
    use super::GodotImmutable;
    use crate::builtin::*;
    use crate::meta::Element;

    unsafe impl GodotImmutable for bool {}
    unsafe impl GodotImmutable for i8 {}
    unsafe impl GodotImmutable for u8 {}
    unsafe impl GodotImmutable for i16 {}
    unsafe impl GodotImmutable for u16 {}
    unsafe impl GodotImmutable for i32 {}
    unsafe impl GodotImmutable for u32 {}
    unsafe impl GodotImmutable for i64 {}
    unsafe impl GodotImmutable for f32 {}
    unsafe impl GodotImmutable for f64 {}

    // No NodePath, Callable, Signal, Rid, Variant.
    unsafe impl GodotImmutable for Aabb {}
    unsafe impl GodotImmutable for Basis {}
    unsafe impl GodotImmutable for Color {}
    unsafe impl GodotImmutable for GString {}
    unsafe impl GodotImmutable for Plane {}
    unsafe impl GodotImmutable for Projection {}
    unsafe impl GodotImmutable for Quaternion {}
    unsafe impl GodotImmutable for Rect2 {}
    unsafe impl GodotImmutable for Rect2i {}
    unsafe impl GodotImmutable for StringName {}
    unsafe impl GodotImmutable for Transform2D {}
    unsafe impl GodotImmutable for Transform3D {}
    unsafe impl GodotImmutable for Vector2 {}
    unsafe impl GodotImmutable for Vector2i {}
    unsafe impl GodotImmutable for Vector3 {}
    unsafe impl GodotImmutable for Vector3i {}
    unsafe impl GodotImmutable for Vector4 {}
    unsafe impl GodotImmutable for Vector4i {}

    unsafe impl GodotImmutable for PackedByteArray {}
    unsafe impl GodotImmutable for PackedColorArray {}
    unsafe impl GodotImmutable for PackedFloat32Array {}
    unsafe impl GodotImmutable for PackedFloat64Array {}
    unsafe impl GodotImmutable for PackedInt32Array {}
    unsafe impl GodotImmutable for PackedInt64Array {}
    unsafe impl GodotImmutable for PackedStringArray {}
    unsafe impl GodotImmutable for PackedVector2Array {}
    unsafe impl GodotImmutable for PackedVector3Array {}
    #[cfg(since_api = "4.3")]
    unsafe impl GodotImmutable for PackedVector4Array {}

    unsafe impl<T> GodotImmutable for Array<T>
    where
        T: GodotImmutable + Element,
    {
        fn into_runtime_immutable(self) -> Self {
            self.into_read_only()
        }
    }
}
