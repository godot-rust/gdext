use std::mem::MaybeUninit;

use gdext_sys::{self as sys, interface_fn};

// Size is dependent on type and build config, can be read from a JSON in the future
#[cfg(not(feature = "real_is_double"))]
const SIZE_IN_BYTES: u64 = 24;
#[cfg(feature = "real_is_double")]
const SIZE_IN_BYTES: u64 = 40;

#[repr(C, align(8))]
pub struct Variant(MaybeUninit<[u8; SIZE_IN_BYTES as usize]>);

impl Variant {
    #[doc(hidden)]
    pub fn uninit() -> Self {
        Self(MaybeUninit::uninit())
    }

    #[doc(hidden)]
    pub fn as_mut_ptr(&mut self) -> sys::GDNativeVariantPtr {
        self.0.as_ptr() as *mut _
    }

    #[doc(hidden)]
    pub fn as_ptr(&self) -> sys::GDNativeVariantPtr {
        self.0.as_ptr() as *mut _
    }

    pub fn nil() -> Self {
        unsafe {
            let mut v = Self::uninit();
            interface_fn!(variant_new_nil)(v.as_mut_ptr());
            v
        }
    }
}

impl Clone for Variant {
    fn clone(&self) -> Self {
        unsafe {
            let mut v = Self::uninit();
            interface_fn!(variant_new_copy)(v.as_mut_ptr(), self.as_ptr());
            v
        }
    }
}

impl Drop for Variant {
    fn drop(&mut self) {
        unsafe {
            interface_fn!(variant_destroy)(self.as_ptr());
        }
    }
}

mod conversions {
    use gdext_sys as sys;
    use crate::{string::GodotString, vector2::Vector2, vector3::Vector3};
    use super::Variant;

    macro_rules! impl_variant_conversions {
        ($T:ty, $from_fn:ident, $to_fn:ident) => {
            impl From<$T> for Variant {
                fn from(value: $T) -> Self {
                    unsafe {
                        let converter = sys::get_cache().$from_fn;

                        let mut variant = Variant::uninit();
                        converter(variant.as_mut_ptr(), &value as *const _ as *mut _);
                        variant
                    }
                }
            }

            impl From<&Variant> for $T {
                fn from(variant: &Variant) -> Self {
                    unsafe {
                        let converter = sys::get_cache().$to_fn;

                        let mut value = <$T>::default();
                        converter(&mut value as *mut _ as *mut _, variant.as_ptr());
                        value
                    }
                }
            }
        }
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
                let converter = sys::get_cache().variant_from_string;

                let mut variant = Variant::uninit();
                converter(variant.as_mut_ptr(), value as *const _ as *mut _);
                variant
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
