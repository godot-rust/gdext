use gdext_sys as sys;
use sys::types::OpaqueVariant;
use sys::{impl_ffi_as_opaque_pointer, interface_fn, GodotFfi};

#[repr(C, align(8))]
pub struct Variant {
    opaque: OpaqueVariant,
}

impl Variant {
    pub fn nil() -> Self {
        unsafe {
            Self::from_sys_init(|ptr| {
                interface_fn!(variant_new_nil)(ptr);
            })
        }
    }

    fn from_opaque(opaque: OpaqueVariant) -> Self {
        Self { opaque }
    }
}

impl Clone for Variant {
    fn clone(&self) -> Self {
        unsafe {
            Self::from_sys_init(|ptr| {
                interface_fn!(variant_new_copy)(ptr, self.sys());
            })
        }
    }
}

impl Drop for Variant {
    fn drop(&mut self) {
        unsafe {
            interface_fn!(variant_destroy)(self.sys_mut());
        }
    }
}

impl GodotFfi for Variant {
    impl_ffi_as_opaque_pointer!(sys::GDNativeVariantPtr);
}

mod conversions {
    use super::Variant;
    use crate::{string::GodotString, vector2::Vector2, vector3::Vector3};
    use gdext_sys as sys;
    use sys::GodotFfi;

    macro_rules! impl_variant_conversions {
        ($T:ty, $from_fn:ident, $to_fn:ident) => {
            impl From<$T> for Variant {
                fn from(value: $T) -> Self {
                    unsafe {
                        Self::from_sys_init(|variant_ptr| {
                            let converter = sys::get_cache().$from_fn;
                            //converter(variant_ptr, &value as *const _ as *mut std::ffi::c_void);
                            converter(variant_ptr, &value as *const _ as sys::GDNativeTypePtr);
                            //converter(variant_ptr, value.sys()); // TODO: use trait?
                        })
                    }
                }
            }

            impl From<&Variant> for $T {
                fn from(variant: &Variant) -> Self {
                    unsafe {
                        let mut value = <$T>::default();

                        let converter = sys::get_cache().$to_fn;
                        //converter(&mut value as *mut _ as *mut std::ffi::c_void, variant.sys());
                        converter(&mut value as *mut _ as sys::GDNativeTypePtr, variant.sys());
                        //converter(value.sys(), variant.sys()); // TODO: use trait?
                        value
                    }
                }
            }
        };
    }

    macro_rules! impl_variant_int_conversions {
        ($name:ty) => {
            impl From<$name> for Variant {
                fn from(i: $name) -> Self {
                    Variant::from(i as i64)
                }
            }

            impl From<&Variant> for $name {
                fn from(i: &Variant) -> Self {
                    i64::from(i) as $name
                }
            }
        };
    }

    impl_variant_conversions!(bool, bool_to_variant, bool_from_variant);
    impl_variant_conversions!(i64, int_to_variant, int_from_variant);
    impl_variant_conversions!(Vector2, vector2_to_variant, vector2_from_variant);
    impl_variant_conversions!(Vector3, vector3_to_variant, vector3_from_variant);
    impl_variant_conversions!(GodotString, string_to_variant, string_from_variant);

    impl_variant_int_conversions!(u8);
    impl_variant_int_conversions!(u16);
    impl_variant_int_conversions!(u32);
    impl_variant_int_conversions!(u64);

    impl_variant_int_conversions!(i8);
    impl_variant_int_conversions!(i16);
    impl_variant_int_conversions!(i32);

    // Strings by ref
    impl From<&GodotString> for Variant {
        fn from(value: &GodotString) -> Self {
            unsafe {
                Self::from_sys_init(|ptr| {
                    let converter = sys::get_cache().string_to_variant;
                    //converter(ptr, &value as *const _ as *mut std::ffi::c_void);
                    converter(ptr, value.sys()); // TODO:CHECK
                })
            }
        }
    }

    // Unit
    impl From<()> for Variant {
        fn from(_unit: ()) -> Self {
            Self::nil()
        }
    }
}
