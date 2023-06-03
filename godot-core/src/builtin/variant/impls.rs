/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use super::*;
use crate::builtin::meta::{PropertyInfo, VariantMetadata};
use crate::builtin::*;
use crate::engine::global;
use crate::obj::EngineEnum;
use godot_ffi as sys;
use sys::GodotFfi;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Macro definitions

macro_rules! impl_variant_metadata {
    ($T:ty, $variant_type:ident $( ; $($extra:tt)* )?) => {
        impl VariantMetadata for $T {
            fn variant_type() -> VariantType {
                VariantType::$variant_type
            }

            $($($extra)*)?
        }
    };
}
// Certain types need to be passed as initialized pointers in their from_variant implementations in 4.0. Because
// 4.0 uses `*ptr = value` to return the type, and some types in c++ override `operator=` in c++ in a way
// that requires the pointer the be initialized. But some other types will cause a memory leak in 4.1 if
// initialized.
//
// Thus we can use `init` to indicate when it must be initialized in 4.0.
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
                // Type check -- at the moment, a strict match is required.
                if variant.get_type() != Self::variant_type() {
                    return Err(VariantConversionError::BadType)
                }

                // For 4.0:
                // In contrast to T -> Variant, the conversion Variant -> T assumes
                // that the destination is initialized (at least for some T). For example:
                // void String::operator=(const String &p_str) { _cowdata._ref(p_str._cowdata); }
                // does a copy-on-write and explodes if this->_cowdata is not initialized.
                // We can thus NOT use Self::from_sys_init().
                //
                // This was changed in 4.1.
                let result = unsafe {
                    sys::from_sys_init_or_init_default(|self_ptr| {
                        let converter = sys::builtin_fn!($to_fn);
                        converter(self_ptr, variant.var_sys());
                    })
                };

                Ok(result)
            }
        }

        impl_variant_metadata!($T, $variant_type; $($extra)*);
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
                    .and_then(|i| <$T>::try_from(i).map_err(|_e| VariantConversionError::BadType))
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
#[allow(clippy::module_inception)]
mod impls {
    use super::*;

    impl_variant_traits!(Aabb, aabb_to_variant, aabb_from_variant, Aabb);
    impl_variant_traits!(bool, bool_to_variant, bool_from_variant, Bool);
    impl_variant_traits!(Basis, basis_to_variant, basis_from_variant, Basis);
    impl_variant_traits!(Callable, callable_to_variant, callable_from_variant, Callable);
    impl_variant_traits!(Vector2, vector2_to_variant, vector2_from_variant, Vector2);
    impl_variant_traits!(Vector3, vector3_to_variant, vector3_from_variant, Vector3);
    impl_variant_traits!(Vector4, vector4_to_variant, vector4_from_variant, Vector4);
    impl_variant_traits!(Vector2i, vector2i_to_variant, vector2i_from_variant, Vector2i);
    impl_variant_traits!(Vector3i, vector3i_to_variant, vector3i_from_variant, Vector3i);
    impl_variant_traits!(Quaternion, quaternion_to_variant, quaternion_from_variant, Quaternion);
    impl_variant_traits!(Color, color_to_variant, color_from_variant, Color);
    impl_variant_traits!(GodotString, string_to_variant, string_from_variant, String);
    impl_variant_traits!(StringName, string_name_to_variant, string_name_from_variant, StringName);
    impl_variant_traits!(NodePath, node_path_to_variant, node_path_from_variant, NodePath);
    // TODO use impl_variant_traits!, as soon as `Default` is available. Also consider auto-generating.
    impl_variant_metadata!(Signal, /* signal_to_variant, signal_from_variant, */ Signal);
    impl_variant_traits!(PackedByteArray, packed_byte_array_to_variant, packed_byte_array_from_variant, PackedByteArray);
    impl_variant_traits!(PackedInt32Array, packed_int32_array_to_variant, packed_int32_array_from_variant, PackedInt32Array);
    impl_variant_traits!(PackedInt64Array, packed_int64_array_to_variant, packed_int64_array_from_variant, PackedInt64Array);
    impl_variant_traits!(PackedFloat32Array, packed_float32_array_to_variant, packed_float32_array_from_variant, PackedFloat32Array);
    impl_variant_traits!(PackedFloat64Array, packed_float64_array_to_variant, packed_float64_array_from_variant, PackedFloat64Array);
    impl_variant_traits!(PackedStringArray, packed_string_array_to_variant, packed_string_array_from_variant, PackedStringArray);
    impl_variant_traits!(PackedVector2Array, packed_vector2_array_to_variant, packed_vector2_array_from_variant, PackedVector2Array);
    impl_variant_traits!(PackedVector3Array, packed_vector3_array_to_variant, packed_vector3_array_from_variant, PackedVector3Array);
    impl_variant_traits!(PackedColorArray, packed_color_array_to_variant, packed_color_array_from_variant, PackedColorArray);
    impl_variant_traits!(Plane, plane_to_variant, plane_from_variant, Plane);
    impl_variant_traits!(Projection, projection_to_variant, projection_from_variant, Projection);
    impl_variant_traits!(Rid, rid_to_variant, rid_from_variant, Rid);
    impl_variant_traits!(Rect2, rect2_to_variant, rect2_from_variant, Rect2);
    impl_variant_traits!(Rect2i, rect2i_to_variant, rect2i_from_variant, Rect2i);
    impl_variant_traits!(Transform2D, transform_2d_to_variant, transform_2d_from_variant, Transform2D);
    impl_variant_traits!(Transform3D, transform_3d_to_variant, transform_3d_from_variant, Transform3D);
    impl_variant_traits!(Dictionary, dictionary_to_variant, dictionary_from_variant, Dictionary);

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
        // Arrays use the `NIL` type to indicate that they are untyped.
        VariantType::Nil
    }

    fn property_info(property_name: &str) -> PropertyInfo {
        PropertyInfo {
            variant_type: Self::variant_type(),
            class_name: Self::class_name(),
            property_name: StringName::from(property_name),
            hint: global::PropertyHint::PROPERTY_HINT_NONE,
            hint_string: GodotString::new(),
            usage: global::PropertyUsageFlags::PROPERTY_USAGE_NIL_IS_VARIANT,
        }
    }

    fn param_metadata() -> sys::GDExtensionClassMethodArgumentMetadata {
        sys::GDEXTENSION_METHOD_ARGUMENT_METADATA_INT_IS_INT8
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
            .and_then(|int| Self::try_from_ord(int).ok_or(VariantConversionError::BadType))
    }
}

impl<T: EngineEnum> VariantMetadata for T {
    fn variant_type() -> VariantType {
        VariantType::Int
    }
    fn param_metadata() -> sys::GDExtensionClassMethodArgumentMetadata {
        sys::GDEXTENSION_METHOD_ARGUMENT_METADATA_INT_IS_INT32
    }
}
