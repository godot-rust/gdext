/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::builtin::meta::{
    impl_godot_as_self, ArrayElement, ConvertError, FromGodot, GodotConvert, GodotType, ToGodot,
};
use crate::builtin::Variant;
use godot_ffi as sys;

// The following ToGodot/FromGodot/Convert impls are auto-generated for each engine type, co-located with their definitions:
// - enum
// - const/mut pointer to native struct

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Option

impl<T: GodotConvert> GodotConvert for Option<T>
where
    Option<T::Via>: GodotType,
{
    type Via = Option<T::Via>;
}

impl<T: ToGodot> ToGodot for Option<T>
where
    Option<T::Via>: GodotType,
{
    fn to_godot(&self) -> Self::Via {
        self.as_ref().map(ToGodot::to_godot)
    }

    fn into_godot(self) -> Self::Via {
        self.map(ToGodot::into_godot)
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

            fn to_ffi(&self) -> Self::Ffi {
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

        impl_godot_scalar!(@shared_traits; $T);
    };

    ($T:ty as $Via:ty, $param_metadata:expr; lossy) => {
        impl GodotType for $T {
            type Ffi = $Via;

            fn to_ffi(&self) -> Self::Ffi {
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
        impl ArrayElement for $T {}

        impl GodotConvert for $T {
            type Via = $T;
        }

        impl ToGodot for $T {
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
impl_godot_as_self!(bool);
impl_godot_as_self!(i64);
impl_godot_as_self!(f64);
impl_godot_as_self!(());

// Also implements ArrayElement.
impl_godot_scalar!(
    i8 as i64,
    crate::builtin::meta::FromFfiError::I8,
    sys::GDEXTENSION_METHOD_ARGUMENT_METADATA_INT_IS_INT8
);
impl_godot_scalar!(
    u8 as i64,
    crate::builtin::meta::FromFfiError::U8,
    sys::GDEXTENSION_METHOD_ARGUMENT_METADATA_INT_IS_UINT8
);
impl_godot_scalar!(
    i16 as i64,
    crate::builtin::meta::FromFfiError::I16,
    sys::GDEXTENSION_METHOD_ARGUMENT_METADATA_INT_IS_INT16
);
impl_godot_scalar!(
    u16 as i64,
    crate::builtin::meta::FromFfiError::U16,
    sys::GDEXTENSION_METHOD_ARGUMENT_METADATA_INT_IS_UINT16
);
impl_godot_scalar!(
    i32 as i64,
    crate::builtin::meta::FromFfiError::I32,
    sys::GDEXTENSION_METHOD_ARGUMENT_METADATA_INT_IS_INT32
);
impl_godot_scalar!(
    u32 as i64,
    crate::builtin::meta::FromFfiError::U32,
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

    fn to_ffi(&self) -> Self::Ffi {
        *self as i64
    }

    fn into_ffi(self) -> Self::Ffi {
        self as i64
    }

    fn try_from_ffi(ffi: Self::Ffi) -> Result<Self, ConvertError> {
        // Ok(ffi as u64)
        Self::try_from(ffi)
            .map_err(|_rust_err| crate::builtin::meta::FromFfiError::U64.into_error(ffi))
    }

    impl_godot_scalar!(@shared_fns; i64, sys::GDEXTENSION_METHOD_ARGUMENT_METADATA_INT_IS_UINT64);
}

impl GodotConvert for u64 {
    type Via = u64;
}

impl ToGodot for u64 {
    fn to_godot(&self) -> Self::Via {
        *self
    }

    fn to_variant(&self) -> Variant {
        // TODO panic doesn't fit the trait's infallibility too well; maybe in the future try_to_godot/try_to_variant() methods are possible.
        i64::try_from(*self)
            .map(|v| v.to_variant())
            .unwrap_or_else(|_| {
                panic!("to_variant(): u64 value {} is not representable inside Variant, which can only store i64 integers", self)
            })
    }
}

impl FromGodot for u64 {
    fn try_from_godot(via: Self::Via) -> Result<Self, ConvertError> {
        Ok(via)
    }

    fn try_from_variant(variant: &Variant) -> Result<Self, ConvertError> {
        // Fail for values that are not representable as u64.
        let value = variant.try_to::<i64>()?;

        u64::try_from(value).map_err(|_rust_err| {
            // TODO maybe use better error enumerator
            crate::builtin::meta::FromVariantError::BadValue.into_error(value)
        })
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Raw pointers

// const void* is used in some APIs like OpenXrApiExtension::transform_from_pose().
// void* is used by ScriptExtension::instance_create().
// Other impls for raw pointers are generated for native structures.

macro_rules! impl_pointer_convert {
    ($Ptr:ty) => {
        impl GodotConvert for $Ptr {
            type Via = i64;
        }

        impl ToGodot for $Ptr {
            fn to_godot(&self) -> Self::Via {
                *self as i64
            }
        }

        impl FromGodot for $Ptr {
            fn try_from_godot(via: Self::Via) -> Result<Self, ConvertError> {
                Ok(via as Self)
            }
        }
    };
}

impl_pointer_convert!(*const std::ffi::c_void);
impl_pointer_convert!(*mut std::ffi::c_void);

// Some other pointer types are used by various other methods, see https://github.com/godot-rust/gdext/issues/677
// TODO: Find better solution to this, this may easily break still if godot decides to add more pointer arguments.

impl_pointer_convert!(*mut *const u8);
impl_pointer_convert!(*mut i32);
impl_pointer_convert!(*mut f64);
impl_pointer_convert!(*mut u8);
