use std::ffi::CStr;

use gdext_builtin::{GodotString, Vector2, Vector3};

pub trait PropertyInfoBuilder {
    fn variant_type() -> gdext_sys::GDNativeVariantType;
    fn property_info(name: &CStr) -> gdext_sys::GDNativePropertyInfo {
        gdext_sys::GDNativePropertyInfo {
            type_: Self::variant_type() as _,
            name: name.as_ptr(),
            class_name: std::ptr::null(),
            hint: 0,
            hint_string: std::ptr::null(),
            usage: 7, // Default, TODO generate global enums
        }
    }
    fn metadata() -> gdext_sys::GDNativeExtensionClassMethodArgumentMetadata {
        gdext_sys::GDNativeExtensionClassMethodArgumentMetadata_GDNATIVE_EXTENSION_METHOD_ARGUMENT_METADATA_NONE
    }
}

impl PropertyInfoBuilder for () {
    fn variant_type() -> gdext_sys::GDNativeVariantType {
        gdext_sys::GDNativeVariantType_GDNATIVE_VARIANT_TYPE_NIL
    }
}

impl PropertyInfoBuilder for GodotString {
    fn variant_type() -> gdext_sys::GDNativeVariantType {
        gdext_sys::GDNativeVariantType_GDNATIVE_VARIANT_TYPE_STRING
    }
}

//
/*
impl PropertyInfoBuilder for &GodotString {
    fn variant_type() -> gdext_sys::GDNativeVariantType {
        gdext_sys::GDNativeVariantType_GDNATIVE_VARIANT_TYPE_STRING
    }
}
*/

impl PropertyInfoBuilder for Vector2 {
    fn variant_type() -> gdext_sys::GDNativeVariantType {
        gdext_sys::GDNativeVariantType_GDNATIVE_VARIANT_TYPE_VECTOR2
    }
}

impl PropertyInfoBuilder for Vector3 {
    fn variant_type() -> gdext_sys::GDNativeVariantType {
        gdext_sys::GDNativeVariantType_GDNATIVE_VARIANT_TYPE_VECTOR3
    }
}

macro_rules! property_info_integer {
    ($type:ty, $meta:ident) => {
        impl PropertyInfoBuilder for $type {
            fn variant_type() -> gdext_sys::GDNativeVariantType {
                gdext_sys::GDNativeVariantType_GDNATIVE_VARIANT_TYPE_INT
            }

            fn metadata() -> gdext_sys::GDNativeExtensionClassMethodArgumentMetadata {
                gdext_sys::$meta
            }
        }
    };
}

property_info_integer!(u8, GDNativeExtensionClassMethodArgumentMetadata_GDNATIVE_EXTENSION_METHOD_ARGUMENT_METADATA_INT_IS_UINT8);
property_info_integer!(u16, GDNativeExtensionClassMethodArgumentMetadata_GDNATIVE_EXTENSION_METHOD_ARGUMENT_METADATA_INT_IS_UINT16);
property_info_integer!(u32, GDNativeExtensionClassMethodArgumentMetadata_GDNATIVE_EXTENSION_METHOD_ARGUMENT_METADATA_INT_IS_UINT32);
property_info_integer!(u64, GDNativeExtensionClassMethodArgumentMetadata_GDNATIVE_EXTENSION_METHOD_ARGUMENT_METADATA_INT_IS_UINT64);

property_info_integer!(i8, GDNativeExtensionClassMethodArgumentMetadata_GDNATIVE_EXTENSION_METHOD_ARGUMENT_METADATA_INT_IS_INT8);
property_info_integer!(i16, GDNativeExtensionClassMethodArgumentMetadata_GDNATIVE_EXTENSION_METHOD_ARGUMENT_METADATA_INT_IS_INT16);
property_info_integer!(i32, GDNativeExtensionClassMethodArgumentMetadata_GDNATIVE_EXTENSION_METHOD_ARGUMENT_METADATA_INT_IS_INT32);
property_info_integer!(i64, GDNativeExtensionClassMethodArgumentMetadata_GDNATIVE_EXTENSION_METHOD_ARGUMENT_METADATA_INT_IS_INT64);
