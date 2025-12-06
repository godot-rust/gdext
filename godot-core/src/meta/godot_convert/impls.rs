/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot_ffi as sys;

use crate::builtin::{Array, Variant};
use crate::meta;
use crate::meta::error::{ConvertError, ErrorKind, FromFfiError};
use crate::meta::{
    ArrayElement, ClassId, FromGodot, GodotConvert, GodotNullableFfi, GodotType, PropertyHintInfo,
    PropertyInfo, ToGodot,
};
use crate::registry::method::MethodParamOrReturnInfo;

// The following ToGodot/FromGodot/Convert impls are auto-generated for each engine type, co-located with their definitions:
// - enum
// - const/mut pointer to native struct

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Option<T>

impl<T> GodotType for Option<T>
where
    T: GodotType,
    T::Ffi: GodotNullableFfi,
    for<'f> T::ToFfi<'f>: GodotNullableFfi,
{
    type Ffi = T::Ffi;

    type ToFfi<'f> = T::ToFfi<'f>;

    fn to_ffi(&self) -> Self::ToFfi<'_> {
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

    fn class_id() -> ClassId {
        T::class_id()
    }

    fn property_info(property_name: &str) -> PropertyInfo {
        T::property_info(property_name)
    }

    fn property_hint_info() -> PropertyHintInfo {
        T::property_hint_info()
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
}

impl<T> ToGodot for Option<T>
where
    // Currently limited to holding objects -> needed to establish to_godot() relation T::to_godot() = Option<&T::Via>.
    T: ToGodot<Pass = meta::ByObject>,
    // Extra Clone bound for to_godot_owned(); might be extracted in the future.
    T::Via: Clone,
    // T::Via must be a Godot nullable type (to support the None case).
    for<'f> T::Via: GodotType<
        // Associated types need to be nullable.
        Ffi: GodotNullableFfi,
        ToFfi<'f>: GodotNullableFfi,
    >,
    // Previously used bound, not needed right now but don't remove: Option<T::Via>: GodotType,
{
    // Basically ByRef, but allows Option<T> -> Option<&T::Via> conversion.
    type Pass = meta::ByOption<T::Via>;

    fn to_godot(&self) -> Option<&T::Via> {
        self.as_ref().map(T::to_godot)
    }

    fn to_godot_owned(&self) -> Option<T::Via>
    where
        Self::Via: Clone,
    {
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

    fn from_variant(variant: &Variant) -> Self {
        if variant.is_nil() {
            return None;
        }

        Some(T::from_variant(variant))
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Scalars

macro_rules! impl_godot_scalar {
    ($T:ty as $Via:ty, $err:path, $param_metadata:expr) => {
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
        impl ArrayElement for $T {
            fn debug_validate_elements(array: &Array<Self>) -> Result<(), ConvertError> {
                array.debug_validate_int_elements()
            }
        }

        impl_godot_scalar!(@shared_traits; $T);
    };

    ($T:ty as $Via:ty, $param_metadata:expr; lossy) => {
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
        impl ArrayElement for $T {}

        impl_godot_scalar!(@shared_traits; $T);
    };

    (@shared_fns; $Via:ty, $param_metadata:expr) => {
        fn param_metadata() -> sys::GDExtensionClassMethodArgumentMetadata {
            $param_metadata
        }

        fn godot_type_name() -> String {
            <$Via as GodotType>::godot_type_name()
        }
    };

    (@shared_traits; $T:ty) => {
        impl GodotConvert for $T {
            type Via = $T;
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

// Also implements ArrayElement.
impl_godot_scalar!(
    i8 as i64,
    FromFfiError::I8,
    sys::GDEXTENSION_METHOD_ARGUMENT_METADATA_INT_IS_INT8
);
impl_godot_scalar!(
    u8 as i64,
    FromFfiError::U8,
    sys::GDEXTENSION_METHOD_ARGUMENT_METADATA_INT_IS_UINT8
);
impl_godot_scalar!(
    i16 as i64,
    FromFfiError::I16,
    sys::GDEXTENSION_METHOD_ARGUMENT_METADATA_INT_IS_INT16
);
impl_godot_scalar!(
    u16 as i64,
    FromFfiError::U16,
    sys::GDEXTENSION_METHOD_ARGUMENT_METADATA_INT_IS_UINT16
);
impl_godot_scalar!(
    i32 as i64,
    FromFfiError::I32,
    sys::GDEXTENSION_METHOD_ARGUMENT_METADATA_INT_IS_INT32
);
impl_godot_scalar!(
    u32 as i64,
    FromFfiError::U32,
    sys::GDEXTENSION_METHOD_ARGUMENT_METADATA_INT_IS_UINT32
);
impl_godot_scalar!(
    f32 as f64,
    sys::GDEXTENSION_METHOD_ARGUMENT_METADATA_REAL_IS_FLOAT;
    lossy
);

// ----------------------------------------------------------------------------------------------------------------------------------------------
// u64: manually implemented, to ensure that type is not altered during conversion.

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

    impl_godot_scalar!(@shared_fns; i64, sys::GDEXTENSION_METHOD_ARGUMENT_METADATA_INT_IS_UINT64);
}

impl GodotConvert for u64 {
    type Via = u64;
}

// u64 implements internal-only conversion traits for use in engine APIs and virtual methods.
impl meta::EngineToGodot for u64 {
    type Pass = meta::ByValue;

    fn engine_to_godot(&self) -> meta::ToArg<'_, Self::Via, Self::Pass> {
        *self
    }

    fn engine_to_variant(&self) -> Variant {
        Variant::from(*self as i64) // Treat as i64.
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
// Collections

impl<T: ArrayElement> GodotConvert for Vec<T> {
    type Via = Array<T>;
}

impl<T: ArrayElement> ToGodot for Vec<T> {
    type Pass = meta::ByValue;

    fn to_godot(&self) -> Self::Via {
        Array::from(self.as_slice())
    }
}

impl<T: ArrayElement> FromGodot for Vec<T> {
    fn try_from_godot(via: Self::Via) -> Result<Self, ConvertError> {
        Ok(via.iter_shared().collect())
    }
}

impl<T: ArrayElement, const LEN: usize> GodotConvert for [T; LEN] {
    type Via = Array<T>;
}

impl<T: ArrayElement, const LEN: usize> ToGodot for [T; LEN] {
    type Pass = meta::ByValue;

    fn to_godot(&self) -> Self::Via {
        Array::from(self)
    }
}

impl<T: ArrayElement, const LEN: usize> FromGodot for [T; LEN] {
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

impl<T: ArrayElement> GodotConvert for &[T] {
    type Via = Array<T>;
}

impl<T: ArrayElement> ToGodot for &[T] {
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
