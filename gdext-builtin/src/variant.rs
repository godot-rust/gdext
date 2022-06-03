use crate::GodotString;
use gdext_sys as sys;
use std::fmt;
use sys::types::OpaqueVariant;
use sys::{impl_ffi_as_opaque_pointer, interface_fn};

#[repr(C, align(8))]
pub struct Variant {
    opaque: OpaqueVariant,
}

impl Variant {
    pub fn nil() -> Self {
        unsafe {
            Self::from_var_sys_init(|ptr| {
                interface_fn!(variant_new_nil)(ptr);
            })
        }
    }

    #[allow(unused_mut)]
    fn stringify(&self) -> GodotString {
        let mut result = GodotString::new();
        unsafe {
            interface_fn!(variant_stringify)(self.var_sys(), result.string_sys());
        }
        result
    }

    fn from_opaque(opaque: OpaqueVariant) -> Self {
        Self { opaque }
    }

    // Conversions from/to Godot C++ `Variant*` pointers
    impl_ffi_as_opaque_pointer!(sys::GDNativeVariantPtr; from_var_sys, from_var_sys_init, var_sys, write_var_sys);
}

impl Clone for Variant {
    fn clone(&self) -> Self {
        unsafe {
            Self::from_var_sys_init(|ptr| {
                interface_fn!(variant_new_copy)(ptr, self.var_sys());
            })
        }
    }
}

impl Drop for Variant {
    fn drop(&mut self) {
        unsafe {
            interface_fn!(variant_destroy)(self.var_sys());
        }
    }
}

impl fmt::Display for Variant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = self.stringify();
        write!(f, "{}", s)
    }
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
                        Self::from_var_sys_init(|variant_ptr| {
                            let converter = sys::get_cache().$from_fn;
                            converter(variant_ptr, value.sys());
                        })
                    }
                }
            }

            impl From<&Variant> for $T {
                fn from(variant: &Variant) -> Self {
                    // In contrast to T -> Variant, the conversion Variant -> T assumes
                    // that the destination is initialized (at least for some T). For example:
                    // void String::operator=(const String &p_str) { _cowdata._ref(p_str._cowdata); }
                    // does a copy-on-write and explodes if this->_cowdata is not initialized.
                    // We can thus NOT use Self::from_sys_init().

                    let mut value = <$T>::default();

                    unsafe {
                        let converter = sys::get_cache().$to_fn;
                        converter(value.sys_mut(), variant.var_sys());
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
    // u64 only TryFrom

    impl_variant_int_conversions!(i8);
    impl_variant_int_conversions!(i16);
    impl_variant_int_conversions!(i32);

    impl TryFrom<u64> for Variant {
        type Error = std::num::TryFromIntError;

        fn try_from(value: u64) -> Result<Self, Self::Error> {
            i64::try_from(value).map(|i| Variant::from(i))
        }
    }

    impl TryFrom<&Variant> for u64 {
        type Error = std::num::TryFromIntError;

        fn try_from(variant: &Variant) -> Result<Self, Self::Error> {
            match i64::try_from(variant) {
                Ok(i) => u64::try_from(i),
                Err(_) => unreachable!(),
            }
        }
    }

    // Strings by ref
    impl From<&GodotString> for Variant {
        fn from(value: &GodotString) -> Self {
            unsafe {
                Self::from_var_sys_init(|ptr| {
                    let converter = sys::get_cache().string_to_variant;
                    converter(ptr, value.sys());
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
    // not possible due to orphan rule
    // impl<T> From<Variant> for T
    // where
    //     T: From<&Variant>,
    // {
    //     fn from(variant: Variant) -> Self {
    //         // same as &Variant, but consume
    //         T::from(&variant)
    //     }
    // }
}
