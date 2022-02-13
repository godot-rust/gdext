#![macro_use]

#[doc(hidden)]
#[macro_export]
macro_rules! gdext_wrap_method_parameter_count {
    () => {
        0
    };
    ($name:ident, $($other:ident,)*) => {
        1 + $crate::gdext_wrap_method_parameter_count!($($other,)*)
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! gdext_wrap_method_has_return_value {
    (()) => {
        false
    };
    ($name:ty) => {
        true
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! gdext_wrap_method_inner {
    (
        $type_name:ty,
        $map_method:ident,
        fn $method_name:ident(
            self
            $(,$pname:ident : $pty:ty)*
            $(, #[opt] $opt_pname:ident : $opt_pty:ty)*
        ) -> $retty:ty
    ) => {
        unsafe {
            const NUM_ARGS: usize = gdext_wrap_method_parameter_count!($($pname,)*);

            let method_info = sys::GDNativeExtensionClassMethodInfo {
                name: concat!(stringify!($method_name), "\0").as_bytes().as_ptr() as *const _,
                method_userdata: std::ptr::null_mut(),
                call_func: Some({
                    unsafe extern "C" fn call(
                        _method_data: *mut std::ffi::c_void,
                        instance: sys::GDExtensionClassInstancePtr,
                        args: *const sys::GDNativeVariantPtr,
                        _arg_count: sys::GDNativeInt,
                        ret: sys::GDNativeVariantPtr,
                        err: *mut sys::GDNativeCallError,
                    ) {
                        let instance = &mut *(instance as *mut $type_name);

                        let mut idx = 0;

                        $(
                            let $pname = <$pty as From<&Variant>>::from(&*(*args.offset(idx) as *mut Variant));
                            idx += 1;
                        )*

                        let ret_val = instance.$method_name($(
                            $pname,
                        )*);
                        *(ret as *mut Variant) = Variant::from(ret_val);

                        (*err).error = sys::GDNativeCallErrorType_GDNATIVE_CALL_OK;
                    }

                    call
                }),
                ptrcall_func: Some({
                    unsafe extern "C" fn call(
                        _method_data: *mut std::ffi::c_void,
                        instance: sys::GDExtensionClassInstancePtr,
                        args: *const sys::GDNativeTypePtr,
                        ret: sys::GDNativeTypePtr,
                    ) {
                        let instance = &mut *(instance as *mut $type_name);
                        let mut idx = 0;

                        $(
                            let $pname = <$pty as gdext_builtin::PtrCallArg>::from_ptr_call_arg(args.offset(idx));
                            idx += 1;
                        )*

                        let ret_val = instance.$method_name($(
                            $pname,
                        )*);
                        <$retty as gdext_builtin::PtrCallArg>::to_ptr_call_arg(ret_val, ret);
                    }

                    call
                }),
                method_flags:
                    sys::GDNativeExtensionClassMethodFlags_GDNATIVE_EXTENSION_METHOD_FLAGS_DEFAULT as _,
                argument_count: NUM_ARGS as _,
                has_return_value: gdext_wrap_method_has_return_value!($retty) as u8,
                get_argument_type_func: Some({
                    extern "C" fn get_type(
                        _method_data: *mut std::ffi::c_void,
                        n: i32,
                    ) -> sys::GDNativeVariantType {
                        // return value first
                        let types: [gdext_sys::GDNativeVariantType; NUM_ARGS + 1] = [
                            <$retty as $crate::property_info::PropertyInfoBuilder>::variant_type(),
                            $(
                                <$pty as $crate::property_info::PropertyInfoBuilder>::variant_type(),
                            )*
                        ];
                        types[(n + 1) as usize]
                    }
                    get_type
                }),
                get_argument_info_func: Some({
                    unsafe extern "C" fn get_info(
                        _method_data: *mut std::ffi::c_void,
                        n: i32,
                        ret: *mut sys::GDNativePropertyInfo,
                    ) {
                        // return value fist
                        let infos: [gdext_sys::GDNativePropertyInfo; NUM_ARGS + 1] = [
                            <$retty as $crate::property_info::PropertyInfoBuilder>::property_info(std::ffi::CStr::from_bytes_with_nul_unchecked("\0".as_bytes())),
                            $(
                                <$pty as $crate::property_info::PropertyInfoBuilder>::property_info(std::ffi::CStr::from_bytes_with_nul_unchecked(concat!(stringify!($pname), "\0").as_bytes())),
                            )*
                        ];

                        *ret = infos[(n + 1) as usize];
                    }
                    get_info
                }),
                get_argument_metadata_func: Some({
                    extern "C" fn get_meta(
                        _method_data: *mut std::ffi::c_void,
                        n: i32,
                    ) -> sys::GDNativeExtensionClassMethodArgumentMetadata {
                        // return value first
                        let metas: [gdext_sys::GDNativeExtensionClassMethodArgumentMetadata; NUM_ARGS + 1] = [
                            <$retty as $crate::property_info::PropertyInfoBuilder>::metadata(),
                            $(
                                <$pty as $crate::property_info::PropertyInfoBuilder>::metadata(),
                            )*
                        ];
                        metas[(n + 1) as usize]
                    }
                    get_meta
                }),
                default_argument_count: 0,
                default_arguments: std::ptr::null_mut(),
            };

            let name = std::ffi::CStr::from_bytes_with_nul_unchecked(concat!(stringify!($type_name), "\0").as_bytes());

            interface_fn!(classdb_register_extension_class_method)(
                gdext_sys::get_library() as *mut _,
                name.as_ptr(),
                &method_info as *const _,
            );

        }
    };
}

/// Convenience macro to wrap an object's method into a function pointer
/// that can be passed to the engine when registering a class.
#[macro_export]
macro_rules! gdext_wrap_method {
    // mutable
    (
        $type_name:ty,
        fn $method_name:ident(
            &mut self
            $(,$pname:ident : $pty:ty)*
            $(,#[opt] $opt_pname:ident : $opt_pty:ty)*
            $(,)?
        ) -> $retty:ty
    ) => {
        $crate::gdext_wrap_method_inner!(
            $type_name,
            map_mut,
            fn $method_name(
                self
                $(,$pname : $pty)*
                $(,#[opt] $opt_pname : $opt_pty)*
            ) -> $retty
        )
    };
    // immutable
    (
        $type_name:ty,
        fn $method_name:ident(
            &self
            $(,$pname:ident : $pty:ty)*
            $(,#[opt] $opt_pname:ident : $opt_pty:ty)*
            $(,)?
        ) -> $retty:ty
    ) => {
        $crate::gdext_wrap_method_inner!(
            $type_name,
            map,
            fn $method_name(
                self
                $(,$pname : $pty)*
                $(,#[opt] $opt_pname : $opt_pty)*
            ) -> $retty
        )
    };
    // mutable without return type
    (
        $type_name:ty,
        fn $method_name:ident(
            &mut self
            $(,$pname:ident : $pty:ty)*
            $(,#[opt] $opt_pname:ident : $opt_pty:ty)*
            $(,)?
        )
    ) => {
        $crate::gdext_wrap_method!(
            $type_name,
            fn $method_name(
                &mut self
                $(,$pname : $pty)*
                $(,#[opt] $opt_pname : $opt_pty)*
            ) -> ()
        )
    };
    // immutable without return type
    (
        $type_name:ty,
        fn $method_name:ident(
            &self
            $(,$pname:ident : $pty:ty)*
            $(,#[opt] $opt_pname:ident : $opt_pty:ty)*
            $(,)?
        )
    ) => {
        $crate::gdext_wrap_method!(
            $type_name,
            fn $method_name(
                &self
                $(,$pname : $pty)*
                $(,#[opt] $opt_pname : $opt_pty)*
            ) -> ()
        )
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! gdext_virtual_method_inner {
    (
        $type_name:ty,
        $map_method:ident,
        fn $method_name:ident(
            self
            $(,$pname:ident : $pty:ty)*
        ) -> $retty:ty
    ) => {
        Some({
            unsafe extern "C" fn call(
                instance: gdext_sys::GDExtensionClassInstancePtr,
                args: *const gdext_sys::GDNativeTypePtr,
                ret: gdext_sys::GDNativeTypePtr,
            ) {
                let instance = &mut *(instance as *mut $type_name);
                let mut idx = 0;

                $(
                    let $pname = <$pty as gdext_builtin::PtrCallArg>::from_ptr_call_arg(args.offset(idx));
                    idx += 1;
                )*

                let ret_val = instance.$method_name($(
                    $pname,
                )*);
                <$retty as gdext_builtin::PtrCallArg>::to_ptr_call_arg(ret_val, ret);
            }
            call
        })
    };
}

#[macro_export]
macro_rules! gdext_virtual_method_body {
    // mutable
    (
        $type_name:ty,
        fn $method_name:ident(
            &mut self
            $(,$pname:ident : $pty:ty)*
            $(,)?
        ) -> $retty:ty
    ) => {
        $crate::gdext_virtual_method_inner!(
            $type_name,
            map_mut,
            fn $method_name(
                self
                $(,$pname : $pty)*
            ) -> $retty
        )
    };
    // immutable
    (
        $type_name:ty,
        fn $method_name:ident(
            &self
            $(,$pname:ident : $pty:ty)*
            $(,)?
        ) -> $retty:ty
    ) => {
        $crate::gdext_virtual_method_inner!(
            $type_name,
            map,
            fn $method_name(
                self
                $(,$pname : $pty)*
            ) -> $retty
        )
    };
    // mutable without return type
    (
        $type_name:ty,
        fn $method_name:ident(
            &mut self
            $(,$pname:ident : $pty:ty)*
            $(,)?
        )
    ) => {
        $crate::gdext_virtual_method_body!(
            $type_name,
            fn $method_name(
                &mut self
                $(,$pname : $pty)*
            ) -> ()
        )
    };
    // immutable without return type
    (
        $type_name:ty,
        fn $method_name:ident(
            &self
            $(,$pname:ident : $pty:ty)*
            $(,)?
        )
    ) => {
        $crate::gdext_virtual_method_body!(
            $type_name,
            fn $method_name(
                &self
                $(,$pname : $pty)*
            ) -> ()
        )
    };
}
