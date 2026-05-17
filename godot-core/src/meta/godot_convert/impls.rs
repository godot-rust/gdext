/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::fmt;

use crate::builtin::{Array, Variant};
use crate::meta;
use crate::meta::error::{
    CallError, CallOutcome, ConvertError, ErrorKind, ErrorToGodot, FromFfiError,
};
use crate::meta::shape::GodotShape;
use crate::meta::{
    Element, EngineToGodot, FromGodot, GodotConvert, GodotNullableType, GodotType, ToGodot,
};
use crate::registry::info::ParamMetadata;

// The following ToGodot/FromGodot/Convert impls are auto-generated for each engine type, co-located with their definitions:
// - enum
// - const/mut pointer to native struct

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Option<T>

impl<T: GodotNullableType> GodotType for Option<T> {
    type Ffi = T::Ffi;
    type ToFfi<'f> = T::ToFfi<'f>;

    fn to_ffi(&self) -> Self::ToFfi<'_> {
        self.as_ref()
            .map(|t| t.to_ffi())
            .unwrap_or_else(T::ffi_null_ref)
    }

    fn into_ffi(self) -> Self::Ffi {
        self.map(|t| t.into_ffi()).unwrap_or_else(T::ffi_null)
    }

    fn try_from_ffi(ffi: Self::Ffi) -> Result<Self, ConvertError> {
        if T::ffi_is_null(&ffi) {
            return Ok(None);
        }

        GodotType::try_from_ffi(ffi).map(Some)
    }

    fn from_ffi(ffi: Self::Ffi) -> Self {
        if T::ffi_is_null(&ffi) {
            return None;
        }

        Some(GodotType::from_ffi(ffi))
    }

    // Only relevant for object types T.
    fn as_object_arg(&self) -> meta::ObjectArg<'_> {
        match self {
            Some(inner) => inner.as_object_arg(),
            None => meta::ObjectArg::null(),
        }
    }
}

impl<T> GodotConvert for Option<T>
where
    T: GodotConvert,
    Option<T::Via>: GodotType,
{
    type Via = Option<T::Via>;

    fn godot_shape() -> GodotShape {
        // Option<Gd<T>> is nullable, so param metadata will return NONE instead of OBJECT_IS_REQUIRED.
        match T::godot_shape() {
            GodotShape::Class {
                class_id, heritage, ..
            } => GodotShape::Class {
                class_id,
                heritage,
                is_nullable: true,
            },
            other => other,
        }
    }
}

impl<T> ToGodot for Option<T>
where
    // Currently limited to holding objects -> needed to establish to_godot() relation T::to_godot() = Option<&T::Via>.
    T: ToGodot<Pass = meta::ByObject>,
    // T::Via must be a Godot nullable type (to support the None case).
    T::Via: GodotNullableType,
    // Previously used bound, not needed right now but don't remove: Option<T::Via>: GodotType,
{
    // Basically ByRef, but allows Option<T> -> Option<&T::Via> conversion.
    type Pass = meta::ByOption<T::Via>;

    fn to_godot(&self) -> Option<&T::Via> {
        self.as_ref().map(T::to_godot)
    }

    fn to_godot_owned(&self) -> Option<T::Via> {
        self.as_ref().map(T::to_godot_owned)
    }

    fn to_variant(&self) -> Variant {
        match self {
            Some(inner) => inner.to_variant(),
            None => Variant::nil(),
        }
    }
}

impl<T: FromGodot> FromGodot for Option<T>
where
    Option<T::Via>: GodotType,
{
    fn try_from_godot(via: Self::Via) -> Result<Self, ConvertError> {
        match via {
            Some(via) => T::try_from_godot(via).map(Some),
            None => Ok(None),
        }
    }

    fn from_godot(via: Self::Via) -> Self {
        via.map(T::from_godot)
    }

    fn try_from_variant(variant: &Variant) -> Result<Self, ConvertError> {
        // Note: this forwards to T::Via, not Self::Via (= Option<T>::Via).
        // For Option<T>, there is a blanket impl GodotType, so case differentiations are not possible.
        if T::Via::qualifies_as_special_none(variant) {
            return Ok(None);
        }

        if variant.is_nil() {
            return Ok(None);
        }

        let value = T::try_from_variant(variant)?;
        Ok(Some(value))
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Result<T, E: ErrorToGodot>

impl<T, E> GodotConvert for Result<T, E>
where
    T: ToGodot,
    E: ErrorToGodot<T>,
{
    type Via = <<E as ErrorToGodot<T>>::Mapped as GodotConvert>::Via;

    fn godot_shape() -> GodotShape {
        <<E as ErrorToGodot<T>>::Mapped>::godot_shape()
    }
}

impl<T, E> EngineToGodot for Result<T, E>
where
    T: ToGodot,
    E: ErrorToGodot<T>,
{
    type Pass = meta::ByValue;

    fn engine_to_godot(&self) -> meta::ToArg<'_, Self::Via, Self::Pass> {
        panic_non_consuming()
    }

    fn engine_to_godot_owned(&self) -> Self::Via {
        panic_non_consuming()
    }

    fn engine_to_variant(&self) -> Variant {
        panic_non_consuming()
    }

    // Varcall and ptrcall each need their own override; merging them would force an extra conversion in one direction.
    //
    // Varcall writes a Variant, so engine_try_into_variant produces one directly -- routing through Via first would add a clone for ByRef types.
    // Ptrcall writes Via, so engine_try_into_godot_owned produces it directly -- routing through Variant would cost a Variant→Via round-trip.
    //
    // Both methods also report unexpected errors as CallError rather than panicking.

    fn engine_try_into_variant(self, call_ctx: &meta::CallContext) -> Result<Variant, CallError> {
        match E::result_to_godot(self) {
            CallOutcome::Return(mapped) => Ok(mapped.to_variant()),
            CallOutcome::CallFailed(msg) => Err(CallError::failed_by_user_result(call_ctx, msg)),
        }
    }

    fn engine_try_into_godot_owned(
        self,
        call_ctx: &meta::CallContext,
    ) -> Result<Self::Via, CallError> {
        match E::result_to_godot(self) {
            CallOutcome::Return(mapped) => Ok(mapped.to_godot_owned()),
            CallOutcome::CallFailed(msg) => Err(CallError::failed_by_user_result(call_ctx, msg)),
        }
    }
}

fn panic_non_consuming() -> ! {
    panic!(
        "Result<T, E> is only valid as a #[func] return value; non-owned conversions unsupported"
    )
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Scalars

macro_rules! impl_godot_scalar {
    ($T:ty as $Via:ty, $err:path, $param_metadata:expr_2021) => {
        impl GodotType for $T {
            type Ffi = $Via;
            type ToFfi<'f> = $Via;

            fn to_ffi(&self) -> Self::ToFfi<'_> {
                (*self).into()
            }

            fn into_ffi(self) -> Self::Ffi {
                self.into()
            }

            fn try_from_ffi(ffi: Self::Ffi) -> Result<Self, ConvertError> {
                Self::try_from(ffi).map_err(|_rust_err| {
                    // rust_err is something like "out of range integral type conversion attempted", not adding extra information.
                    // TODO consider passing value into error message, but how thread-safely? don't eagerly convert to string.
                    $err.into_error(ffi)
                })
            }

            impl_godot_scalar!(@shared_fns; $Via, $param_metadata);
        }

        // For integer types, we can validate the conversion.
        impl Element for $T {
            fn debug_validate_elements(array: &Array<Self>) -> Result<(), ConvertError> {
                array.debug_validate_int_elements()
            }
        }

        impl_godot_scalar!(@shared_traits; $T);
    };

    ($T:ty as $Via:ty, $param_metadata:expr_2021; lossy) => {
        impl GodotType for $T {
            type Ffi = $Via;
            type ToFfi<'f> = $Via;

            fn to_ffi(&self) -> Self::ToFfi<'_> {
                *self as $Via
            }

            fn into_ffi(self) -> Self::Ffi {
                self as $Via
            }

            fn try_from_ffi(ffi: Self::Ffi) -> Result<Self, ConvertError> {
                Ok(ffi as $T)
            }

            impl_godot_scalar!(@shared_fns; $Via, $param_metadata);
        }

        // For f32, conversion from f64 is lossy but will always succeed. Thus no debug validation needed.
        impl Element for $T {}

        impl_godot_scalar!(@shared_traits; $T);
    };

    (@shared_fns; $Via:ty, $param_metadata:expr_2021) => {
        fn default_metadata() -> ParamMetadata {
            $param_metadata
        }
    };

    (@shared_traits; $T:ty) => {
        impl GodotConvert for $T {
            type Via = $T;

            fn godot_shape() -> GodotShape {
                GodotShape::of_builtin::<$T>()
            }
        }

        impl ToGodot for $T {
            type Pass = meta::ByValue;

            fn to_godot(&self) -> Self::Via {
               *self
            }
        }

        impl FromGodot for $T {
            fn try_from_godot(via: Self::Via) -> Result<Self, ConvertError> {
                Ok(via)
            }
        }
    };
}

// `GodotType` for these three is implemented in `godot-core/src/builtin/variant/impls.rs`.
meta::impl_godot_as_self!(bool: ByValue);
meta::impl_godot_as_self!(i64: ByValue);
meta::impl_godot_as_self!(f64: ByValue);
meta::impl_godot_as_self!((): ByValue);

// Also implements Element.
impl_godot_scalar!(i8 as i64, FromFfiError::I8, ParamMetadata::INT_IS_INT8);
impl_godot_scalar!(u8 as i64, FromFfiError::U8, ParamMetadata::INT_IS_UINT8);
impl_godot_scalar!(i16 as i64, FromFfiError::I16, ParamMetadata::INT_IS_INT16);
impl_godot_scalar!(u16 as i64, FromFfiError::U16, ParamMetadata::INT_IS_UINT16);
impl_godot_scalar!(i32 as i64, FromFfiError::I32, ParamMetadata::INT_IS_INT32);
impl_godot_scalar!(u32 as i64, FromFfiError::U32, ParamMetadata::INT_IS_UINT32);
impl_godot_scalar!(f32 as f64, ParamMetadata::REAL_IS_FLOAT; lossy);

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Lossy-tier integers (u64, usize): can exceed i64 range, no ToGodot/FromGodot impls.
// Engine-side conversion only; user-facing access gated by #[func(lossy)].
//
// Output direction (Rust → Godot int): out-of-range surfaces as CallError via engine_try_into_* overrides.
// Input direction (Godot int → Rust):
//   - usize: target-aware checked. wasm32 (32-bit usize) rejects > u32::MAX and negatives; 64-bit targets reject only negatives.
//   - u64: bit-reinterpret (i64 as u64). Required by engine APIs/bitfields (i64 wire carries raw bit pattern).
//          Under #[func(lossy)], negative GDScript ints therefore become large u64 values — documented, matches engine API usage.

/// Builds a CallError for return-side overflow of a lossy-tier integer that doesn't fit in i64.
fn lossy_overflow_err<T>(value: impl fmt::Display, call_ctx: &meta::CallContext) -> CallError {
    let type_name = std::any::type_name::<T>();
    let msg = format!("{type_name} value {value} does not fit in i64 (Godot int)");
    CallError::failed_return_conversion::<T>(
        call_ctx,
        ConvertError::with_kind_value(ErrorKind::Custom(Some(msg.into())), ()),
    )
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// u64: FFI wire is i64; C++ reinterprets bits as uint64_t in engine APIs/bitfields.

impl GodotType for u64 {
    type Ffi = i64;
    type ToFfi<'f> = i64;

    fn to_ffi(&self) -> Self::ToFfi<'_> {
        *self as i64
    }

    fn into_ffi(self) -> Self::Ffi {
        self as i64
    }

    fn try_from_ffi(ffi: Self::Ffi) -> Result<Self, ConvertError> {
        Ok(ffi as u64)
    }

    impl_godot_scalar!(@shared_fns; i64, ParamMetadata::INT_IS_UINT64);
}

impl GodotConvert for u64 {
    type Via = u64;

    fn godot_shape() -> GodotShape {
        GodotShape::of_builtin::<u64>()
    }
}

impl EngineToGodot for u64 {
    type Pass = meta::ByValue;

    // Non-try methods reinterpret bits (engine API contract). #[func(lossy)] codegen calls the try_ overrides, which reject values > i64::MAX.

    fn engine_to_godot(&self) -> meta::ToArg<'_, Self::Via, Self::Pass> {
        *self
    }

    fn engine_to_variant(&self) -> Variant {
        (*self as i64).to_variant() // Treat as i64 (bit-reinterpret for engine APIs/bitfields).
    }

    fn engine_try_into_variant(self, call_ctx: &meta::CallContext) -> Result<Variant, CallError> {
        i64::try_from(self)
            .map(|i| i.to_variant())
            .map_err(|_| lossy_overflow_err::<u64>(self, call_ctx))
    }

    fn engine_try_into_godot_owned(
        self,
        call_ctx: &meta::CallContext,
    ) -> Result<Self::Via, CallError> {
        // Via is u64; on success bit pattern fits non-negative i64, so the original `self` round-trips byte-equal through FFI.
        i64::try_from(self)
            .map(|_| self)
            .map_err(|_| lossy_overflow_err::<u64>(self, call_ctx))
    }
}

impl meta::EngineFromGodot for u64 {
    fn engine_try_from_godot(via: Self::Via) -> Result<Self, ConvertError> {
        Ok(via)
    }

    fn engine_try_from_variant(variant: &Variant) -> Result<Self, ConvertError> {
        variant.try_to::<i64>().map(|i| i as u64)
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// usize: pointer-width. wasm32 (Godot Web) = 32-bit, elsewhere typically 64-bit. Variant int is always 64-bit, so failure mode flips per target.

impl GodotConvert for usize {
    type Via = i64;

    fn godot_shape() -> GodotShape {
        GodotShape::of_builtin::<i64>()
    }
}

impl EngineToGodot for usize {
    type Pass = meta::ByValue;

    // Non-try methods are defensive: usize has no Var/signal/Array impls, so the only live paths are #[func(lossy)] varcall/ptrcall via the
    // try_ overrides below. If reached anyway, panic rather than silently truncate (same precedent as Result<T, E>).

    fn engine_to_godot(&self) -> meta::ToArg<'_, Self::Via, Self::Pass> {
        i64::try_from(*self)
            .unwrap_or_else(|_| panic!("usize value {self} does not fit in i64 (Godot int)"))
    }

    fn engine_to_variant(&self) -> Variant {
        <Self as EngineToGodot>::engine_to_godot(self).to_variant()
    }

    fn engine_try_into_variant(self, call_ctx: &meta::CallContext) -> Result<Variant, CallError> {
        i64::try_from(self)
            .map(|i| i.to_variant())
            .map_err(|_| lossy_overflow_err::<usize>(self, call_ctx))
    }

    fn engine_try_into_godot_owned(
        self,
        call_ctx: &meta::CallContext,
    ) -> Result<Self::Via, CallError> {
        i64::try_from(self).map_err(|_| lossy_overflow_err::<usize>(self, call_ctx))
    }
}

impl meta::EngineFromGodot for usize {
    fn engine_try_from_godot(via: Self::Via) -> Result<Self, ConvertError> {
        // usize::try_from(i64) is target-aware: wasm32 rejects > u32::MAX and negatives; 64-bit targets reject only negatives.
        usize::try_from(via).map_err(|_| {
            ConvertError::with_kind_value(
                ErrorKind::Custom(Some(
                    format!("i64 value {via} does not fit in usize on this target").into(),
                )),
                via,
            )
        })
    }

    fn engine_try_from_variant(variant: &Variant) -> Result<Self, ConvertError> {
        let via = variant.try_to::<i64>()?;
        <Self as meta::EngineFromGodot>::engine_try_from_godot(via)
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Collections

impl<T: Element> GodotConvert for Vec<T> {
    type Via = Array<T>;

    fn godot_shape() -> GodotShape {
        <Array<T> as GodotConvert>::godot_shape()
    }
}

impl<T: Element> ToGodot for Vec<T> {
    type Pass = meta::ByValue;

    fn to_godot(&self) -> Self::Via {
        Array::from(self.as_slice())
    }
}

impl<T: Element> FromGodot for Vec<T> {
    fn try_from_godot(via: Self::Via) -> Result<Self, ConvertError> {
        Ok(via.iter_shared().collect())
    }
}

impl<T: Element, const LEN: usize> GodotConvert for [T; LEN] {
    type Via = Array<T>;

    fn godot_shape() -> GodotShape {
        <Array<T> as GodotConvert>::godot_shape()
    }
}

impl<T: Element, const LEN: usize> ToGodot for [T; LEN] {
    type Pass = meta::ByValue;

    fn to_godot(&self) -> Self::Via {
        Array::from(self)
    }
}

impl<T: Element, const LEN: usize> FromGodot for [T; LEN] {
    fn try_from_godot(via: Self::Via) -> Result<Self, ConvertError> {
        let via_len = via.len(); // Caching this avoids an FFI call
        if via_len != LEN {
            let message =
                format!("Array<T> of length {via_len} cannot be stored in [T; {LEN}] Rust array");
            return Err(ConvertError::with_kind_value(
                ErrorKind::Custom(Some(message.into())),
                via,
            ));
        }

        let mut option_array = [const { None }; LEN];

        for (element, destination) in via.iter_shared().zip(&mut option_array) {
            *destination = Some(element);
        }

        let array = option_array.map(|some| {
            some.expect(
                "Elements were removed from Array during `iter_shared()`, this is not allowed",
            )
        });

        Ok(array)
    }
}

impl<T: Element> GodotConvert for &[T] {
    type Via = Array<T>;

    fn godot_shape() -> GodotShape {
        <Array<T> as GodotConvert>::godot_shape()
    }
}

impl<T: Element> ToGodot for &[T] {
    type Pass = meta::ByValue;

    fn to_godot(&self) -> Self::Via {
        Array::from(*self)
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Raw pointers

// Following types used to be manually implemented, but are now covered by RawPtr<P>.
// - *mut *const u8
// - *mut i32
// - *mut f64
// - *mut u8
// - *const u8
//
// *const c_void: is used in some APIs like OpenXrApiExtension::transform_from_pose().
// *mut c_void: is used by ScriptExtension::instance_create().
//
// Other impls for raw pointers are generated for native structures and sys pointers (e.g. GDExtensionManager::load_extension_from_function).
// Some other pointer types are used by various other methods, see https://github.com/godot-rust/gdext/issues/677

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Tests for ToGodot/FromGodot missing impls
//
// Sanity check: comment-out ::godot::meta::ensure_func_bounds in func.rs, the 3 latter #[func] ones should fail.

/// Test that `u64` cannot be converted to variant.
///
/// ```compile_fail
/// # use godot::prelude::*;
/// let variant = 100u64.to_variant();  // Error: u64 does not implement ToGodot
/// ```
fn __doctest_u64() {}

/// Test that `*mut i32` cannot be converted to variant.
///
/// ```compile_fail
/// # use godot::prelude::*;
/// let ptr: *mut i32 = std::ptr::null_mut();
/// let variant = ptr.to_variant();  // Error: *mut i32 does not implement ToGodot
/// ```
fn __doctest_i32_ptr_to_variant() {}

/// Test that void-pointers cannot be converted from variant.
///
/// ```compile_fail
/// # use godot::prelude::*;
/// let variant = Variant::nil();
/// let ptr: *const std::ffi::c_void = variant.to();
/// ```
fn __doctest_void_ptr_from_variant() {}

/// Test that native struct pointers cannot be used as `#[func]` parameters.
///
/// ```compile_fail
/// # use godot::prelude::*;
/// # use godot::classes::native::AudioFrame;
/// #[derive(GodotClass)]
/// #[class(init)]
/// struct MyClass {}
///
/// #[godot_api]
/// impl MyClass {
///     #[func]
///     fn take_pointer(&self, ptr: *mut AudioFrame) {}
/// }
/// ```
fn __doctest_native_struct_pointer_param() {}

/// Test that native struct pointers cannot be used as `#[func]` return types.
///
/// ```compile_fail
/// # use godot::prelude::*;
/// # use godot::classes::native::AudioFrame;
/// #[derive(GodotClass)]
/// #[class(init)]
/// struct MyClass {}
///
/// #[godot_api]
/// impl MyClass {
///     #[func]
///     fn return_pointer(&self) -> *const AudioFrame {
///         std::ptr::null()
///     }
/// }
/// ```
fn __doctest_native_struct_pointer_return() {}

/// Test that `u64` cannot be returned from `#[func]`.
///
/// ```compile_fail
/// # use godot::prelude::*;
/// #[derive(GodotClass)]
/// #[class(init)]
/// struct MyClass {}
///
/// #[godot_api]
/// impl MyClass {
///     #[func]
///     fn return_pointer(&self) -> u64 { 123 }
/// }
/// ```
fn __doctest_u64_return() {}
