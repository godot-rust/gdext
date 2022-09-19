use crate::obj::InstanceId;
use gdext_builtin::{GodotString, Vector2, Vector3};
use gdext_sys as sys;

pub trait PropertyInfoBuilder {
    fn variant_type() -> sys::GDNativeVariantType;

    fn property_info(property_name: &str) -> sys::GDNativePropertyInfo {
        let reg = unsafe { sys::get_registry() };
        sys::GDNativePropertyInfo {
            type_: Self::variant_type() as u32,
            name: reg.c_string(property_name),
            class_name: std::ptr::null_mut(),
            hint: 0,
            hint_string: std::ptr::null_mut(),
            usage: 7, // Default, TODO generate global enums
        }
    }

    fn param_metadata() -> sys::GDNativeExtensionClassMethodArgumentMetadata {
        sys::GDNativeExtensionClassMethodArgumentMetadata_GDNATIVE_EXTENSION_METHOD_ARGUMENT_METADATA_NONE
    }
}

impl PropertyInfoBuilder for () {
    fn variant_type() -> sys::GDNativeVariantType {
        sys::GDNativeVariantType_GDNATIVE_VARIANT_TYPE_NIL
    }
}

impl PropertyInfoBuilder for bool {
    fn variant_type() -> sys::GDNativeVariantType {
        sys::GDNativeVariantType_GDNATIVE_VARIANT_TYPE_BOOL
    }
}

impl PropertyInfoBuilder for GodotString {
    fn variant_type() -> sys::GDNativeVariantType {
        sys::GDNativeVariantType_GDNATIVE_VARIANT_TYPE_STRING
    }
}

//
/*
impl PropertyInfoBuilder for &GodotString {
    fn variant_type() -> sys::GDNativeVariantType {
        sys::GDNativeVariantType_GDNATIVE_VARIANT_TYPE_STRING
    }
}
*/

impl PropertyInfoBuilder for Vector2 {
    fn variant_type() -> sys::GDNativeVariantType {
        sys::GDNativeVariantType_GDNATIVE_VARIANT_TYPE_VECTOR2
    }
}

impl PropertyInfoBuilder for Vector3 {
    fn variant_type() -> sys::GDNativeVariantType {
        sys::GDNativeVariantType_GDNATIVE_VARIANT_TYPE_VECTOR3
    }
}

macro_rules! property_info_integer {
    ($type:ty, $meta:ident) => {
        impl PropertyInfoBuilder for $type {
            fn variant_type() -> sys::GDNativeVariantType {
                sys::GDNativeVariantType_GDNATIVE_VARIANT_TYPE_INT
            }

            fn param_metadata() -> sys::GDNativeExtensionClassMethodArgumentMetadata {
                sys::$meta
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

property_info_integer!(InstanceId, GDNativeExtensionClassMethodArgumentMetadata_GDNATIVE_EXTENSION_METHOD_ARGUMENT_METADATA_INT_IS_INT64);

// ----------------------------------------------------------------------------------------------------------------------------------------------

trait SignatureTuple {
    fn variant_type(index: usize) -> sys::GDNativeVariantType;
    fn param_metadata(index: usize) -> sys::GDNativeExtensionClassMethodArgumentMetadata;
    fn property_info(index: usize, param_name: &str) -> sys::GDNativePropertyInfo;
}

// impl<P, const N: usize> Sig for [P; N]
// impl<P, T0> Sig for (T0)
// where P: PropertyInfoBuilder {
//     fn variant_type(index: usize) -> sys::GDNativeVariantType {
//           Self[index]::
//     }
//
//     fn param_metadata(index: usize) -> sys::GDNativeExtensionClassMethodArgumentMetadata {
//         todo!()
//     }
//
//     fn property_info(index: usize, param_name: &str) -> sys::GDNativePropertyInfo {
//         todo!()
//     }
// }
//

macro_rules! impl_signature_for_tuple {
    ($($Ty:ident : $n:literal),+) => {
        impl<$($Ty,)+> SignatureTuple for ($($Ty,)+)
            where $( $Ty : PropertyInfoBuilder, )+
        {
            fn variant_type(index: usize) -> sys::GDNativeVariantType {
                match index {
                    $(
                        $n => $Ty::variant_type(),
                    )+
                    _ => unreachable!("variant_type: unavailable for index {}", index),
                    //_ => sys::GDNativeVariantType_GDNATIVE_VARIANT_TYPE_NIL
                }
            }

            fn param_metadata(index: usize) -> sys::GDNativeExtensionClassMethodArgumentMetadata {
                match index {
                    $(
                        $n => $Ty::param_metadata(),
                    )+
                    _ => unreachable!("param_metadata: unavailable for index {}", index),
                }
            }

            fn property_info(index: usize, param_name: &str) -> sys::GDNativePropertyInfo {
                match index {
                    $(
                        $n => $Ty::property_info(param_name),
                    )+
                    _ => unreachable!("property_info: unavailable for index {}", index),
                }
            }
        }
    };
}

impl_signature_for_tuple!(R: 0);
impl_signature_for_tuple!(R: 0, P0: 1);
impl_signature_for_tuple!(R: 0, P0: 1, P1: 2);
impl_signature_for_tuple!(R: 0, P0: 1, P1: 2, P2: 3);
impl_signature_for_tuple!(R: 0, P0: 1, P1: 2, P2: 3, P3: 4);
impl_signature_for_tuple!(R: 0, P0: 1, P1: 2, P2: 3, P3: 4, P4: 5);
impl_signature_for_tuple!(R: 0, P0: 1, P1: 2, P2: 3, P3: 4, P4: 5, P5: 6);
impl_signature_for_tuple!(R: 0, P0: 1, P1: 2, P2: 3, P3: 4, P4: 5, P5: 6, P6: 7);
impl_signature_for_tuple!(R: 0, P0: 1, P1: 2, P2: 3, P3: 4, P4: 5, P5: 6, P6: 7, P7: 8);
impl_signature_for_tuple!(R: 0, P0: 1, P1: 2, P2: 3, P3: 4, P4: 5, P5: 6, P6: 7, P7: 8, P8: 9);
impl_signature_for_tuple!(R: 0, P0: 1, P1: 2, P2: 3, P3: 4, P4: 5, P5: 6, P6: 7, P7: 8, P8: 9, P9: 10);
