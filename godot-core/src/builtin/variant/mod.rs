/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::builtin::GodotString;
use godot_ffi as sys;
use std::fmt;
use sys::types::OpaqueVariant;
use sys::{ffi_methods, interface_fn};

mod variant_traits;

pub use variant_traits::*;

#[repr(C, align(8))]
pub struct Variant {
    opaque: OpaqueVariant,
}

impl Variant {
    pub fn nil() -> Self {
        unsafe {
            Self::from_var_sys_init(|variant_ptr| {
                interface_fn!(variant_new_nil)(variant_ptr);
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
    ffi_methods! {
        type sys::GDNativeVariantPtr = *mut Opaque;

        fn from_var_sys = from_sys;
        fn from_var_sys_init = from_sys_init;
        fn var_sys = sys;
        fn write_var_sys = write_sys;
    }
}

impl Clone for Variant {
    fn clone(&self) -> Self {
        unsafe {
            Self::from_var_sys_init(|variant_ptr| {
                interface_fn!(variant_new_copy)(variant_ptr, self.var_sys());
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

impl fmt::Debug for Variant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // TODO include variant type name
        let s = self.stringify();
        write!(f, "Variant({})", s)
    }
}

mod conversions {
    use super::*;
    use crate::builtin::*;
    use godot_ffi as sys;
    use sys::GodotFfi;

    macro_rules! impl_variant_conversions {
        ($T:ty, $from_fn:ident, $to_fn:ident) => {
            impl ToVariant for $T {
                fn to_variant(&self) -> Variant {
                    let variant = unsafe {
                        Variant::from_var_sys_init(|variant_ptr| {
                            let converter = sys::method_table().$from_fn;
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
                        let converter = sys::method_table().$to_fn;
                        converter(value.sys_mut(), variant.var_sys());
                        value
                    };

                    Ok(result)
                }
            }
        };
    }

    macro_rules! impl_variant_int_conversions {
        ($T:ty) => {
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
        };
    }

    impl_variant_conversions!(bool, bool_to_variant, bool_from_variant);
    impl_variant_conversions!(i64, int_to_variant, int_from_variant);
    impl_variant_conversions!(f64, float_to_variant, float_from_variant);
    impl_variant_conversions!(Vector2, vector2_to_variant, vector2_from_variant);
    impl_variant_conversions!(Vector3, vector3_to_variant, vector3_from_variant);
    impl_variant_conversions!(Vector4, vector4_to_variant, vector4_from_variant);
    impl_variant_conversions!(Vector2i, vector2i_to_variant, vector2i_from_variant);
    impl_variant_conversions!(Vector3i, vector3i_to_variant, vector3i_from_variant);
    impl_variant_conversions!(GodotString, string_to_variant, string_from_variant);

    impl_variant_int_conversions!(u8);
    impl_variant_int_conversions!(u16);
    impl_variant_int_conversions!(u32);
    // u64 is fallible; see below

    impl_variant_int_conversions!(i8);
    impl_variant_int_conversions!(i16);
    impl_variant_int_conversions!(i32);

    /*impl ToVariant for u64 {
        fn try_to_variant(&self) -> Result<Variant, VariantConversionError> {
            i64::try_from(*self)
                .map(|i| i.to_variant())
                .map_err(|_e| VariantConversionError)
        }
    }

    impl FromVariant for u64 {
        fn try_from_variant(variant: &Variant) -> Result<Self, VariantConversionError> {
            match i64::try_from_variant(variant) {
                Ok(i) => u64::try_from(i).map_err(|_e| VariantConversionError),
                Err(_) => unreachable!(),
            }
        }
    }*/

    // f32
    impl ToVariant for f32 {
        fn to_variant(&self) -> Variant {
            let double = *self as f64;
            f64::to_variant(&double)
        }
    }

    impl FromVariant for f32 {
        fn try_from_variant(v: &Variant) -> Result<Self, VariantConversionError> {
            f64::try_from_variant(v).map(|double| double as f32)
        }
    }

    // Strings by ref
    impl From<&GodotString> for Variant {
        fn from(value: &GodotString) -> Self {
            unsafe {
                Self::from_var_sys_init(|variant_ptr| {
                    let converter = sys::method_table().string_to_variant;
                    converter(variant_ptr, value.sys());
                })
            }
        }
    }

    // Unit
    impl ToVariant for () {
        fn to_variant(&self) -> Variant {
            Variant::nil()
        }
    }

    // not possible due to orphan rule
    // impl<T> From<Variant> for T
    // where
    //     T: FromVariant,
    // {
    //     fn from(variant: Variant) -> Self {
    //         // same as &Variant, but consume
    //         T::from(&variant)
    //     }
    // }
}
