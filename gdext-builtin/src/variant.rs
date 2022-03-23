use gdext_sys::types::OpaqueVariant;
use gdext_sys::{self as sys, interface_fn};

#[repr(C, align(8))]
pub struct Variant {
    opaque: OpaqueVariant,
}

impl Variant {
    pub fn nil() -> Self {
        let opaque = unsafe {
            OpaqueVariant::with_init(|ptr| {
                interface_fn!(variant_new_nil)(ptr);
            })
        };

        Self { opaque }
    }

    #[doc(hidden)]
    pub fn from_sys(opaque: OpaqueVariant) -> Self {
        Self { opaque }
    }

    #[doc(hidden)]
    pub fn as_mut_ptr(&mut self) -> sys::GDNativeVariantPtr {
        self.opaque.to_sys_mut()
    }

    #[doc(hidden)]
    pub fn as_ptr(&self) -> sys::GDNativeVariantPtr {
        self.opaque.to_sys()
    }
}

impl Clone for Variant {
    fn clone(&self) -> Self {
        let opaque = unsafe {
            OpaqueVariant::with_init(|ptr| {
                interface_fn!(variant_new_copy)(ptr, self.as_ptr());
            })
        };
        Self { opaque }
    }
}

impl Drop for Variant {
    fn drop(&mut self) {
        unsafe {
            interface_fn!(variant_destroy)(self.opaque.to_sys_mut());
        }
    }
}

mod conversions {
    use super::Variant;
    use crate::{string::GodotString, vector2::Vector2, vector3::Vector3};
    use gdext_sys as sys;

    macro_rules! impl_variant_conversions {
        ($T:ty, $from_fn:ident, $to_fn:ident) => {
            impl From<$T> for Variant {
                fn from(value: $T) -> Self {
                    unsafe {
                        let opaque = $crate::sys::types::OpaqueVariant::with_init(|ptr| {
                            let converter = sys::get_cache().$from_fn;
                            converter(ptr, &value as *const _ as *mut _);
                        });

                        Self { opaque }
                    }
                }
            }

            impl From<&Variant> for $T {
                fn from(variant: &Variant) -> Self {
                    unsafe {
                        let mut value = <$T>::default();

                        let converter = sys::get_cache().$to_fn;
                        converter(&mut value as *mut _ as *mut _, variant.as_ptr());
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

    impl_variant_conversions!(bool, variant_from_bool, variant_to_bool);
    impl_variant_conversions!(i64, variant_from_int, variant_to_int);
    impl_variant_conversions!(Vector2, variant_from_vector2, variant_to_vector2);
    impl_variant_conversions!(Vector3, variant_from_vector3, variant_to_vector3);
    impl_variant_conversions!(GodotString, variant_from_string, variant_to_string);

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
                let opaque = sys::types::OpaqueVariant::with_init(|ptr| {
                    let converter = sys::get_cache().variant_from_string;
                    converter(ptr, &value as *const _ as *mut _);
                });

                Self { opaque }
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
