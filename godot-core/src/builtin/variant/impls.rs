/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use super::*;
use crate::builtin::meta::VariantMetadata;
use crate::builtin::*;
use crate::obj::EngineEnum;
use godot_ffi as sys;
use sys::GodotFfi;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Macro definitions

macro_rules! impl_variant_traits {
    ($T:ty, $from_fn:ident, $to_fn:ident, $variant_type:ident) => {
        impl_variant_traits!(@@ $T, $from_fn, $to_fn, $variant_type;);
    };

    ($T:ty, $from_fn:ident, $to_fn:ident, $variant_type:ident, $param_metadata:ident) => {
        impl_variant_traits!(@@ $T, $from_fn, $to_fn, $variant_type;
            fn param_metadata() -> sys::GDExtensionClassMethodArgumentMetadata {
                sys::$param_metadata
            }
        );
    };

    (@@ $T:ty, $from_fn:ident, $to_fn:ident, $variant_type:ident; $($extra:tt)*) => {
        impl ToVariant for $T {
            fn to_variant(&self) -> Variant {
                let variant = unsafe {
                    Variant::from_var_sys_init(|variant_ptr| {
                        let converter = sys::builtin_fn!($from_fn);
                        converter(variant_ptr, self.sys());
                    })
                };

                variant
            }
        }

        impl FromVariant for $T {
            fn try_from_variant(variant: &Variant) -> Result<Self, VariantConversionError> {
                // In contrast to T -> Variant, the conversion Variant -> T assumes
                // that the destination is initialized (at least for some T). For example:
                // void String::operator=(const String &p_str) { _cowdata._ref(p_str._cowdata); }
                // does a copy-on-write and explodes if this->_cowdata is not initialized.
                // We can thus NOT use Self::from_sys_init().

                let mut value = <$T>::default();
                let result = unsafe {
                    let converter = sys::builtin_fn!($to_fn);
                    converter(value.sys_mut(), variant.var_sys());
                    value
                };

                Ok(result)
            }
        }

        impl VariantMetadata for $T {
            fn variant_type() -> VariantType {
                VariantType::$variant_type
            }

            $($extra)*
        }
    };
}

macro_rules! impl_variant_traits_int {
    ($T:ty, $param_metadata:ident) => {
        impl ToVariant for $T {
            fn to_variant(&self) -> Variant {
                i64::from(*self).to_variant()
            }
        }

        impl FromVariant for $T {
            fn try_from_variant(v: &Variant) -> Result<Self, VariantConversionError> {
                i64::try_from_variant(v)
                    .and_then(|i| <$T>::try_from(i).map_err(|_e| VariantConversionError))
            }
        }

        impl VariantMetadata for $T {
            fn variant_type() -> VariantType {
                VariantType::Int
            }

            fn param_metadata() -> sys::GDExtensionClassMethodArgumentMetadata {
                sys::$param_metadata
            }
        }
    };
}

macro_rules! impl_variant_traits_float {
    ($T:ty, $param_metadata:ident) => {
        impl ToVariant for $T {
            fn to_variant(&self) -> Variant {
                let double = *self as f64;
                f64::to_variant(&double)
            }
        }

        impl FromVariant for $T {
            fn try_from_variant(v: &Variant) -> Result<Self, VariantConversionError> {
                f64::try_from_variant(v).map(|double| double as $T)
            }
        }

        impl VariantMetadata for $T {
            fn variant_type() -> VariantType {
                VariantType::Float
            }

            fn param_metadata() -> sys::GDExtensionClassMethodArgumentMetadata {
                sys::$param_metadata
            }
        }
    };
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// General impls

#[rustfmt::skip]
mod impls {
    use super::*;

    impl_variant_traits!(bool, bool_to_variant, bool_from_variant, Bool);
    impl_variant_traits!(Vector2, vector2_to_variant, vector2_from_variant, Vector2);
    impl_variant_traits!(Vector3, vector3_to_variant, vector3_from_variant, Vector3);
    impl_variant_traits!(Vector4, vector4_to_variant, vector4_from_variant, Vector4);
    impl_variant_traits!(Vector2i, vector2i_to_variant, vector2i_from_variant, Vector2i);
    impl_variant_traits!(Vector3i, vector3i_to_variant, vector3i_from_variant, Vector3i);
    impl_variant_traits!(Color, color_to_variant, color_from_variant, Color);
    impl_variant_traits!(GodotString, string_to_variant, string_from_variant, String);
    impl_variant_traits!(StringName, string_name_to_variant, string_name_from_variant, StringName);


    impl_variant_traits!(i64, int_to_variant, int_from_variant, Int, GDEXTENSION_METHOD_ARGUMENT_METADATA_INT_IS_INT64);
    impl_variant_traits_int!(i8, GDEXTENSION_METHOD_ARGUMENT_METADATA_INT_IS_INT8);
    impl_variant_traits_int!(i16, GDEXTENSION_METHOD_ARGUMENT_METADATA_INT_IS_INT16);
    impl_variant_traits_int!(i32, GDEXTENSION_METHOD_ARGUMENT_METADATA_INT_IS_INT32);

    impl_variant_traits_int!(u8, GDEXTENSION_METHOD_ARGUMENT_METADATA_INT_IS_UINT8);
    impl_variant_traits_int!(u16, GDEXTENSION_METHOD_ARGUMENT_METADATA_INT_IS_UINT16);
    impl_variant_traits_int!(u32, GDEXTENSION_METHOD_ARGUMENT_METADATA_INT_IS_UINT32);
    // u64 is not supported, because it cannot be represented on GDScript side, and implicitly converting to i64 is error-prone.

    impl_variant_traits!(f64, float_to_variant, float_from_variant, Float, GDEXTENSION_METHOD_ARGUMENT_METADATA_REAL_IS_DOUBLE);
    impl_variant_traits_float!(f32, GDEXTENSION_METHOD_ARGUMENT_METADATA_REAL_IS_FLOAT);
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Explicit impls

// Unit
impl ToVariant for () {
    fn to_variant(&self) -> Variant {
        Variant::nil()
    }
}

impl VariantMetadata for () {
    fn variant_type() -> VariantType {
        VariantType::Nil
    }
}

impl ToVariant for Variant {
    fn to_variant(&self) -> Variant {
        self.clone()
    }
}

impl FromVariant for Variant {
    fn try_from_variant(variant: &Variant) -> Result<Self, VariantConversionError> {
        Ok(variant.clone())
    }
}

// Variant itself
impl VariantMetadata for Variant {
    fn variant_type() -> VariantType {
        VariantType::Nil // FIXME is this correct? what else to use? is this called at all?
    }
}

impl<T: EngineEnum> ToVariant for T {
    fn to_variant(&self) -> Variant {
        <i32 as ToVariant>::to_variant(&self.ord())
    }
}

impl<T: EngineEnum> FromVariant for T {
    fn try_from_variant(variant: &Variant) -> Result<Self, VariantConversionError> {
        <i32 as FromVariant>::try_from_variant(variant)
            .and_then(|int| Self::try_from_ord(int).ok_or(VariantConversionError))
    }
}
