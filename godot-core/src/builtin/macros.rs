/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

#![macro_use]

macro_rules! impl_basic_trait_as_sys {
    ( Drop for $Type:ty => $gd_method:ident ) => {
        impl Drop for $Type {
            #[inline]
            fn drop(&mut self) {
                unsafe { (get_api().$gd_method)(self.sys_mut()) }
            }
        }
    };

    ( Clone for $Type:ty => $gd_method:ident ) => {
        impl Clone for $Type {
            #[inline]
            fn clone(&self) -> Self {
                unsafe {
                    let mut result = sys::$GdType::default();
                    (get_api().$gd_method)(&mut result, self.sys());
                    <$Type>::from_sys(result)
                }
            }
        }
    };

    ( Default for $Type:ty => $gd_method:ident ) => {
        impl Default for $Type {
            #[inline]
            fn default() -> Self {
                unsafe {
                    let mut gd_val = sys::$GdType::default();
                    (get_api().$gd_method)(&mut gd_val);
                    <$Type>::from_sys(gd_val)
                }
            }
        }
    };

    ( PartialEq for $Type:ty => $gd_method:ident ) => {
        impl PartialEq for $Type {
            #[inline]
            fn eq(&self, other: &Self) -> bool {
                unsafe {
                    let operator = godot_ffi::method_table().$gd_method;

                    let mut result: bool = false;
                    operator(self.sys(), other.sys(), result.sys_mut());
                    result
                }
            }
        }
    };

    ( Eq for $Type:ty => $gd_method:ident ) => {
		impl_basic_trait_as_sys!(PartialEq for $Type => $gd_method);
        impl Eq for $Type {}
    };

    ( PartialOrd for $Type:ty => $gd_method:ident ) => {
        impl PartialOrd for $Type {
            #[inline]
            fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                let op_less = |lhs, rhs| unsafe {
                    let operator = godot_ffi::method_table().$gd_method;

                    let mut result: bool = false;
                    operator(lhs, rhs, result.sys_mut());
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
        impl_basic_trait_as_sys!(PartialOrd for $Type => $gd_method);
        impl Ord for $Type {
            #[inline]
            fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                PartialOrd::partial_cmp(self, other).expect("PartialOrd::partial_cmp")
            }
        }
    };
}

macro_rules! impl_traits_as_sys {
    (
        for $Type:ty {
            $( $Trait:ident => $gd_method:ident; )*
        }
    ) => (
        $(
            impl_basic_trait_as_sys!(
                $Trait for $Type => $gd_method
            );
        )*
    )
}

macro_rules! impl_builtin_stub {
    ($Class:ident, $OpaqueTy:ident) => {
        #[repr(C)]
        pub struct $Class {
            opaque: sys::types::$OpaqueTy,
        }

        impl $Class {
            fn from_opaque(opaque: sys::types::$OpaqueTy) -> Self {
                Self { opaque }
            }
        }

        impl GodotFfi for $Class {
            ffi_methods! { type sys::GDNativeTypePtr = *mut Opaque; .. }
        }
    };
}

macro_rules! impl_builtin_froms {
    ($To:ty; $($From:ty => $from_fn:ident),* $(,)?) => {
        $(impl From<&$From> for $To {
            fn from(other: &$From) -> Self {
                unsafe {
                    Self::from_sys_init(|ptr| {
                        let converter = sys::method_table().$from_fn;
                        converter(ptr, [other.sys()].as_ptr());
                    })
                }
            }
        })*
    };
}
