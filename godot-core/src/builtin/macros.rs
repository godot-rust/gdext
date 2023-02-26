/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

#![macro_use]

macro_rules! impl_builtin_traits_inner {
    ( Default for $Type:ty => $gd_method:ident ) => {
        impl Default for $Type {
            #[inline]
            fn default() -> Self {
                // Note: can't use from_sys_init(), as that calls the default constructor
                // (because most assignments expect initialized target type)

                let mut uninit = std::mem::MaybeUninit::<$Type>::uninit();

                unsafe {
                    let self_ptr = (*uninit.as_mut_ptr()).sys_mut();
                    sys::builtin_call! {
                        $gd_method(self_ptr, std::ptr::null_mut())
                    };

                    uninit.assume_init()
                }
            }
        }
    };

    ( Clone for $Type:ty => $gd_method:ident ) => {
        impl Clone for $Type {
            #[inline]
            fn clone(&self) -> Self {
                unsafe {
                    Self::from_sys_init_default(|self_ptr| {
                        let ctor = ::godot_ffi::builtin_fn!($gd_method);
                        let args = [self.sys_const()];
                        ctor(self_ptr, args.as_ptr());
                    })
                }
            }
        }
    };

    ( Drop for $Type:ty => $gd_method:ident ) => {
        impl Drop for $Type {
            #[inline]
            fn drop(&mut self) {
                unsafe {
                    let destructor = ::godot_ffi::builtin_fn!($gd_method @1);
                    destructor(self.sys_mut());
                }
            }
        }
    };

    ( PartialEq for $Type:ty => $gd_method:ident ) => {
        impl PartialEq for $Type {
            #[inline]
            fn eq(&self, other: &Self) -> bool {
                unsafe {
                    let mut result = false;
                    ::godot_ffi::builtin_call! {
                        $gd_method(self.sys(), other.sys(), result.sys_mut())
                    };
                    result
                }
            }
        }
    };

    ( Eq for $Type:ty => $gd_method:ident ) => {
        impl_builtin_traits_inner!(PartialEq for $Type => $gd_method);
        impl Eq for $Type {}
    };

    ( PartialOrd for $Type:ty => $gd_method:ident ) => {
        impl PartialOrd for $Type {
            #[inline]
            fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                let op_less = |lhs, rhs| unsafe {
                    let mut result = false;
                    ::godot_ffi::builtin_call! {
                        $gd_method(lhs, rhs, result.sys_mut())
                    };
                    result
                };

                if op_less(self.sys(), other.sys()) {
                    Some(std::cmp::Ordering::Less)
                } else if op_less(other.sys(), self.sys()) {
                    Some(std::cmp::Ordering::Greater)
                } else {
                    Some(std::cmp::Ordering::Equal)
                }
            }
        }
    };

    ( Ord for $Type:ty => $gd_method:ident ) => {
        impl_builtin_traits_inner!(PartialOrd for $Type => $gd_method);
        impl Ord for $Type {
            #[inline]
            fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                PartialOrd::partial_cmp(self, other).expect("PartialOrd::partial_cmp")
            }
        }
    };

    // TODO remove; use godot-core/src/builtin/variant/impls.rs instead (this one is only used for Callable)
    ( FromVariant for $Type:ty => $gd_method:ident ) => {
        impl $crate::builtin::variant::FromVariant for $Type {
            fn try_from_variant(variant: &$crate::builtin::Variant) -> Result<Self, $crate::builtin::variant::VariantConversionError> {
                if variant.get_type() != <Self as $crate::builtin::meta::VariantMetadata>::variant_type() {
                    return Err($crate::builtin::variant::VariantConversionError)
                }
                let result = unsafe {
                    Self::from_sys_init_default(|self_ptr| {
                        let converter = sys::builtin_fn!($gd_method);
                        converter(self_ptr, variant.var_sys());
                    })
                };

                Ok(result)
            }
        }
    };
}

macro_rules! impl_builtin_traits {
    (
        for $Type:ty {
            $( $Trait:ident => $gd_method:ident; )*
        }
    ) => (
        $(
            impl_builtin_traits_inner! {
                $Trait for $Type => $gd_method
            }
        )*
    )
}

macro_rules! impl_builtin_stub {
    // ($Class:ident, $OpaqueTy:ident $( ; )? $( $Traits:ident ),* ) => {
    ($Class:ident, $OpaqueTy:ident) => {
        #[repr(C)]
        // #[derive(Copy, Clone)]
        pub struct $Class {
            opaque: sys::types::$OpaqueTy,
        }

        impl $Class {
            fn from_opaque(opaque: sys::types::$OpaqueTy) -> Self {
                Self { opaque }
            }
        }

        impl GodotFfi for $Class {
            ffi_methods! { type sys::GDExtensionTypePtr = *mut Opaque; .. }
        }
    };
}

macro_rules! impl_builtin_froms {
    ($To:ty; $($From:ty => $from_fn:ident),* $(,)?) => {
        $(impl From<&$From> for $To {
            fn from(other: &$From) -> Self {
                unsafe {
                    Self::from_sys_init(|ptr| {
                        let args = [other.sys_const()];
                        ::godot_ffi::builtin_call! {
                            $from_fn(ptr, args.as_ptr())
                        }
                    })
                }
            }
        })*
    };
}
