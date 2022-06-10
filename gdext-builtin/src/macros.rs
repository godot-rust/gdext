#![macro_use]

#[macro_export]
macro_rules! gdext_print_warning {
    ($($args:tt),* $(,)?) => {
        unsafe {
            let msg = format!("{}\0", format_args!($($args,)*));

            gdext_sys::interface_fn!(print_warning)(
                msg.as_bytes().as_ptr() as *const _,
                "<function unset>\0".as_bytes().as_ptr() as *const _,
                concat!(file!(), "\0").as_ptr() as *const _,
                line!() as _,
            );
        }
    };
}

#[macro_export]
macro_rules! gdext_print_error {
    ($($args:tt),* $(,)?) => {
        unsafe {
            let msg = format!("{}\0", format_args!($($args,)*));

            gdext_sys::interface_fn!(print_error)(
                msg.as_bytes().as_ptr() as *const _,
                "<function unset>\0".as_bytes().as_ptr() as *const _,
                concat!(file!(), "\0").as_ptr() as *const _,
                line!() as _,
            );
        }
    };
}

#[macro_export]
macro_rules! gdext_print_script_error {
    ($($args:tt),* $(,)?) => {
        unsafe {
            let msg = format!("{}\0", format_args!($($args,)*));

            gdext_sys::interface_fn!(print_script_error)(
                msg.as_bytes().as_ptr() as *const _,
                "<function unset>\0".as_bytes().as_ptr() as *const _,
                concat!(file!(), "\0").as_ptr() as *const _,
                line!() as _,
            );
        }
    };
}

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
                    let operator = gdext_sys::method_table().$gd_method;

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
                    let operator = gdext_sys::method_table().$gd_method;

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
                PartialOrd::partial_cmp(self, other).unwrap()
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
