/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use super::*;
use crate::builtin::meta::{GodotFfiVariant, GodotType, PropertyInfo};
use crate::builtin::*;
use crate::engine::global;
use godot_ffi as sys;
use sys::GodotFfi;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Macro definitions

// Certain types need to be passed as initialized pointers in their from_variant implementations in 4.0. Because
// 4.0 uses `*ptr = value` to return the type, and some types in c++ override `operator=` in c++ in a way
// that requires the pointer the be initialized. But some other types will cause a memory leak in 4.1 if
// initialized.
//
// Thus we can use `init` to indicate when it must be initialized in 4.0.
macro_rules! impl_ffi_variant {
    ($T:ty, $from_fn:ident, $to_fn:ident $(; $godot_type_name:ident)?) => {
        impl GodotFfiVariant for $T {
            fn ffi_to_variant(&self) -> Variant {
                let variant = unsafe {
                    Variant::from_var_sys_init(|variant_ptr| {
                        let converter = sys::builtin_fn!($from_fn);
                        converter(variant_ptr, self.sys());
                    })
                };

                variant
            }

            fn ffi_from_variant(variant: &Variant) -> Result<Self, VariantConversionError> {
                // Type check -- at the moment, a strict match is required.
                if variant.get_type() != Self::variant_type() {
                    return Err(VariantConversionError::BadType);
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

        impl GodotType for $T {
            type Ffi = Self;

            fn to_ffi(&self) -> Self::Ffi {
                self.clone()
            }

            fn into_ffi(self) -> Self::Ffi {
                self
            }

            fn try_from_ffi(ffi: Self::Ffi) -> Option<Self> {
                Some(ffi)
            }

            impl_ffi_variant!(@godot_type_name $T $(, $godot_type_name)?);
        }
    };

    (@godot_type_name $T:ty) => {
        fn godot_type_name() -> String {
            stringify!($T).into()
        }
    };

    (@godot_type_name $T:ty, $godot_type_name:ident) => {
        fn godot_type_name() -> String {
            stringify!($godot_type_name).into()
        }
    };
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// General impls

#[rustfmt::skip]
#[allow(clippy::module_inception)]
mod impls {
    use super::*;

    impl_ffi_variant!(Aabb, aabb_to_variant, aabb_from_variant; AABB);
    impl_ffi_variant!(bool, bool_to_variant, bool_from_variant);
    impl_ffi_variant!(Basis, basis_to_variant, basis_from_variant);
    impl_ffi_variant!(Callable, callable_to_variant, callable_from_variant);
    impl_ffi_variant!(Vector2, vector2_to_variant, vector2_from_variant);
    impl_ffi_variant!(Vector3, vector3_to_variant, vector3_from_variant);
    impl_ffi_variant!(Vector4, vector4_to_variant, vector4_from_variant);
    impl_ffi_variant!(Vector2i, vector2i_to_variant, vector2i_from_variant);
    impl_ffi_variant!(Vector3i, vector3i_to_variant, vector3i_from_variant);
    impl_ffi_variant!(Vector4i, vector4i_to_variant, vector4i_from_variant);
    impl_ffi_variant!(Quaternion, quaternion_to_variant, quaternion_from_variant);
    impl_ffi_variant!(Color, color_to_variant, color_from_variant);
    impl_ffi_variant!(GodotString, string_to_variant, string_from_variant; String);
    impl_ffi_variant!(StringName, string_name_to_variant, string_name_from_variant);
    impl_ffi_variant!(NodePath, node_path_to_variant, node_path_from_variant);
    impl_ffi_variant!(PackedByteArray, packed_byte_array_to_variant, packed_byte_array_from_variant);
    impl_ffi_variant!(PackedInt32Array, packed_int32_array_to_variant, packed_int32_array_from_variant);
    impl_ffi_variant!(PackedInt64Array, packed_int64_array_to_variant, packed_int64_array_from_variant);
    impl_ffi_variant!(PackedFloat32Array, packed_float32_array_to_variant, packed_float32_array_from_variant);
    impl_ffi_variant!(PackedFloat64Array, packed_float64_array_to_variant, packed_float64_array_from_variant);
    impl_ffi_variant!(PackedStringArray, packed_string_array_to_variant, packed_string_array_from_variant);
    impl_ffi_variant!(PackedVector2Array, packed_vector2_array_to_variant, packed_vector2_array_from_variant);
    impl_ffi_variant!(PackedVector3Array, packed_vector3_array_to_variant, packed_vector3_array_from_variant);
    impl_ffi_variant!(PackedColorArray, packed_color_array_to_variant, packed_color_array_from_variant);
    impl_ffi_variant!(Plane, plane_to_variant, plane_from_variant);
    impl_ffi_variant!(Projection, projection_to_variant, projection_from_variant);
    impl_ffi_variant!(Rid, rid_to_variant, rid_from_variant; RID);
    impl_ffi_variant!(Rect2, rect2_to_variant, rect2_from_variant);
    impl_ffi_variant!(Rect2i, rect2i_to_variant, rect2i_from_variant);
    impl_ffi_variant!(Signal, signal_to_variant, signal_from_variant);
    impl_ffi_variant!(Transform2D, transform_2d_to_variant, transform_2d_from_variant);
    impl_ffi_variant!(Transform3D, transform_3d_to_variant, transform_3d_from_variant);
    impl_ffi_variant!(Dictionary, dictionary_to_variant, dictionary_from_variant);
    impl_ffi_variant!(i64, int_to_variant, int_from_variant; int);
    impl_ffi_variant!(f64, float_to_variant, float_from_variant; float);
    
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Explicit impls

// Unit
impl GodotFfiVariant for () {
    fn ffi_to_variant(&self) -> Variant {
        Variant::nil()
    }

    fn ffi_from_variant(variant: &Variant) -> Result<Self, VariantConversionError> {
        if variant.is_nil() {
            return Ok(());
        }

        Err(VariantConversionError::BadValue)
    }
}

impl GodotType for () {
    type Ffi = Self;

    fn to_ffi(&self) -> Self::Ffi {}

    fn into_ffi(self) -> Self::Ffi {}

    fn try_from_ffi(_: Self::Ffi) -> Option<Self> {
        Some(())
    }

    fn godot_type_name() -> String {
        "Variant".into()
    }
}

impl GodotFfiVariant for Variant {
    fn ffi_to_variant(&self) -> Variant {
        self.clone()
    }

    fn ffi_from_variant(variant: &Variant) -> Result<Self, VariantConversionError> {
        Ok(variant.clone())
    }
}

impl GodotType for Variant {
    type Ffi = Variant;

    fn to_ffi(&self) -> Self::Ffi {
        self.clone()
    }

    fn into_ffi(self) -> Self::Ffi {
        self
    }

    fn try_from_ffi(ffi: Self::Ffi) -> Option<Self> {
        Some(ffi)
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

    fn godot_type_name() -> String {
        "Variant".into()
    }
}
