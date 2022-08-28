#![macro_use]

#[macro_export]
macro_rules! c_str {
    ($str:literal) => {
        (concat!($str, "\0")).as_ptr() as *const std::os::raw::c_char
    };
}

// TODO code duplication ((2))
#[doc(hidden)]
#[macro_export]
macro_rules! gdext_count_idents {
    () => {
        0
    };
    ($name:ident, $($other:ident,)*) => {
        1 + $crate::gdext_count_idents!($($other,)*)
    }
}

#[doc(hidden)]
#[macro_export]
// Note: this only works if the _argument_ (not parameter) is not a :ty, otherwise it will always match the 2nd branch
macro_rules! gdext_is_not_unit {
    (()) => {
        false
    };
    ($name:ty) => {
        true
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! gdext_register_method_inner {
    (
        $Class:ty,
        $map_method:ident,
        fn $method_name:ident(
            $( $param:ident : $ParamTy:ty, )*
            $( #[opt] $opt_param:ident : $OptParamTy:ty, )*
        ) -> $($RetTy:tt)+ // Note: can't be ty, as that cannot be matched to tokens anymore
    ) => {
        unsafe {
            use gdext_sys as sys;
            use gdext_builtin::Variant;

            const NUM_ARGS: usize = $crate::gdext_count_idents!($( $param, )*);

            let method_info = sys::GDNativeExtensionClassMethodInfo {
                name: concat!(stringify!($method_name), "\0").as_ptr() as *const i8,
                method_userdata: std::ptr::null_mut(),
                call_func: Some({
                    unsafe extern "C" fn function(
                        _method_data: *mut std::ffi::c_void,
                        instance_ptr: sys::GDExtensionClassInstancePtr,
                        args: *const sys::GDNativeVariantPtr,
                        _arg_count: sys::GDNativeInt,
                        ret: sys::GDNativeVariantPtr,
                        err: *mut sys::GDNativeCallError,
                    ) {
                        $crate::gdext_varcall!(
                            instance_ptr, args, ret, err;
                            $Class;
                            fn $method_name( $( $param: $ParamTy, )* ) -> $( $RetTy )+
                        );
                    }

                    function
                }),
                ptrcall_func: Some({
                    unsafe extern "C" fn function(
                        _method_data: *mut std::ffi::c_void,
                        instance_ptr: sys::GDExtensionClassInstancePtr,
                        args: *const sys::GDNativeTypePtr,
                        ret: sys::GDNativeTypePtr,
                    ) {
                        $crate::gdext_ptrcall!(
                            instance_ptr, args, ret;
                            $Class;
                            fn $method_name( $( $param: $ParamTy, )* ) -> $( $RetTy )+
                        );
                    }

                    function
                }),
                method_flags:
                    sys::GDNativeExtensionClassMethodFlags_GDNATIVE_EXTENSION_METHOD_FLAGS_DEFAULT as u32,
                argument_count: NUM_ARGS as u32,
                has_return_value: $crate::gdext_is_not_unit!($($RetTy)+) as u8,
                get_argument_type_func: Some({
                    extern "C" fn get_type(
                        _method_data: *mut std::ffi::c_void,
                        n: i32,
                    ) -> sys::GDNativeVariantType {
                        // Return value is the first "argument"
                        let types: [sys::GDNativeVariantType; NUM_ARGS + 1] = [
                            <$($RetTy)+ as $crate::property_info::PropertyInfoBuilder>::variant_type(),
                            $(
                                <$ParamTy as $crate::property_info::PropertyInfoBuilder>::variant_type(),
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
                        // Return value is the first "argument"
                        let infos: [sys::GDNativePropertyInfo; NUM_ARGS + 1] = [
                            <$($RetTy)+ as $crate::property_info::PropertyInfoBuilder>::property_info(""),
                            $(
                                <$ParamTy as $crate::property_info::PropertyInfoBuilder>::property_info(stringify!($param)),
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
                        // Return value is the first "argument"
                        let metas: [sys::GDNativeExtensionClassMethodArgumentMetadata; NUM_ARGS + 1] = [
                            <$($RetTy)+ as $crate::property_info::PropertyInfoBuilder>::metadata(),
                            $(
                                <$ParamTy as $crate::property_info::PropertyInfoBuilder>::metadata(),
                            )*
                        ];
                        metas[(n + 1) as usize]
                    }
                    get_meta
                }),
                default_argument_count: 0,
                default_arguments: std::ptr::null_mut(),
            };

            let name = std::ffi::CStr::from_bytes_with_nul_unchecked(concat!(stringify!($Class), "\0").as_bytes());

            sys::interface_fn!(classdb_register_extension_class_method)(
                sys::get_library(),
                name.as_ptr(),
                std::ptr::addr_of!(method_info),
            );
        }
    };
}

/// Convenience macro to wrap an object's method into a function pointer
/// that can be passed to the engine when registering a class.
//
// Note: code duplicated with gdext_virtual_method_callback
#[doc(hidden)]
#[macro_export]
macro_rules! gdext_register_method {
    // mutable
    (
        $Class:ty,
        fn $method_name:ident(
            &mut self
            $(, $param:ident : $ParamTy:ty)*
            $(, #[opt] $opt_param:ident : $OptParamTy:ty)*
            $(,)?
        ) -> $RetTy:ty;
    ) => {
        $crate::gdext_register_method_inner!(
            $Class,
            map_mut,
            fn $method_name(
                $(, $param : $ParamTy)*
                $(, #[opt] $opt_param : $OptParamTy)*
            ) -> $RetTy
        )
    };

    // immutable
    (
        $Class:ty,
        fn $method_name:ident(
            &self
            $(, $arg:ident : $Param:ty)*
            $(, #[opt] $opt_arg:ident : $OptParam:ty)*
            $(,)?
        ) -> $RetTy:ty;
    ) => {
        $crate::gdext_register_method_inner!(
            $Class,
            map,
            fn $method_name(
                $( $arg : $Param, )*
                $( #[opt] $opt_arg : $OptParam, )*
            ) -> $RetTy
        )
    };

    // mutable without return type
    (
        $Class:ty,
        fn $method_name:ident(
            &mut self
            $(, $param:ident : $ParamTy:ty )*
            $(, #[opt] $opt_param:ident : $OptParamTy:ty )*
            $(,)?
        );
    ) => {
        // recurse this macro
        $crate::gdext_register_method!(
            $Class,
            fn $method_name(
                &mut self,
                $(, $param : $ParamTy )*
                $(, #[opt] $opt_param : $OptParamTy )*
            ) -> ();
        )
    };

    // immutable without return type
    (
        $Class:ty,
        fn $method_name:ident(
            &self
            $(, $param:ident : $ParamTy:ty )*
            $(, #[opt] $opt_param:ident : $OptParamTy:ty )*
            $(,)?
        );
    ) => {
        // recurse this macro
        $crate::gdext_register_method!(
            $Class,
            fn $method_name(
                &self
                $(, $param : $ParamTy )*
                $(, #[opt] $opt_param : $OptParamTy )*
            ) -> ();
        )
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! gdext_virtual_method_callback_inner {
    (
        $Class:ty,
        $map_method:ident,
        fn $method_name:ident(
            $( $arg:ident : $Param:ty, )*
        ) -> $Ret:ty
    ) => {
        Some({
            use gdext_sys as sys;

            unsafe extern "C" fn function(
                instance_ptr: sys::GDExtensionClassInstancePtr,
                args: *const sys::GDNativeTypePtr,
                ret: sys::GDNativeTypePtr,
            ) {
                $crate::gdext_ptrcall!(
                    instance_ptr, args, ret;
                    $Class;
                    fn $method_name( $( $arg : $Param, )* ) -> $Ret
                );
            }
            function
        })
    };
}

/// Returns a C function which acts as the callback when a virtual method of this instance is invoked.
//
// Note: code duplicated with gdext_virtual_method_callback
#[doc(hidden)]
#[macro_export]
macro_rules! gdext_virtual_method_callback {
    // mutable
    (
        $Class:ty,
        fn $method_name:ident(
            &mut self
            $(, $param:ident : $ParamTy:ty)*
            $(,)?
        ) -> $RetTy:ty
    ) => {
        $crate::gdext_virtual_method_callback_inner!(
            $Class,
            map_mut,
            fn $method_name(
                $( $param : $ParamTy, )*
            ) -> $RetTy
        )
    };

    // immutable
    (
        $Class:ty,
        fn $method_name:ident(
            &self
            $(, $param:ident : $ParamTy:ty)*
            $(,)?
        ) -> $RetTy:ty
    ) => {
        $crate::gdext_virtual_method_callback_inner!(
            $Class,
            map,
            fn $method_name(
                $( $param : $ParamTy, )*
            ) -> $RetTy
        )
    };

    // mutable without return type
    (
        $Class:ty,
        fn $method_name:ident(
            &mut self
            $(, $param:ident : $ParamTy:ty)*
            $(,)?
        )
    ) => {
        // recurse this macro
        $crate::gdext_virtual_method_callback!(
            $Class,
            fn $method_name(
                &mut self
                $(, $param : $ParamTy )*
            ) -> ()
        )
    };

    // immutable without return type
    (
        $Class:ty,
        fn $method_name:ident(
            &self
            $(, $param:ident : $ParamTy:ty)*
            $(,)?
        )
    ) => {
        // recurse this macro
        $crate::gdext_virtual_method_callback!(
            $Class,
            fn $method_name(
                &self
                $(, $param : $ParamTy )*
            ) -> ()
        )
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! gdext_ptrcall {
    (
        $instance_ptr:ident, $args:ident, $ret:ident;
        $Class:ty;
        fn $method_name:ident(
            $( $arg:ident : $ParamTy:ty, )*
        ) -> $( $RetTy:tt )+
    ) => {
        println!("ptrcall: {}", stringify!($method_name));
        let storage = ::gdext_class::private::as_storage::<$Class>($instance_ptr);
        let instance = storage.get_mut_lateinit();

        let mut idx = 0;
        $(
            let $arg = <$ParamTy as sys::GodotFfi>::from_sys(*$args.offset(idx));
            // FIXME update refcount, e.g. Obj::ready() or T::Mem::maybe_inc_ref(&result);
            // possibly in from_sys() directly; what about from_sys_init() and from_{obj|str}_sys()?
            idx += 1;
        )*

        let ret_val = instance.$method_name($(
            $arg,
        )*);

        <$($RetTy)+ as sys::GodotFfi>::write_sys(&ret_val, $ret);
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! gdext_varcall {
    (
        $instance_ptr:ident, $args:ident, $ret:ident, $err:ident;
        $Class:ty;
        fn $method_name:ident(
            $( $arg:ident : $ParamTy:ty, )*
        ) -> $( $RetTy:tt )+
    ) => {
        println!("varcall: {}", stringify!($method_name));
        let storage = ::gdext_class::private::as_storage::<$Class>($instance_ptr);
        let instance = storage.get_mut_lateinit();

        let mut idx = 0;
        $(
            let $arg = <$ParamTy as From<&Variant>>::from(&*(*$args.offset(idx) as *mut Variant));
            idx += 1;
        )*

        let ret_val = instance.$method_name($(
            $arg,
        )*);

        *($ret as *mut Variant) = Variant::from(ret_val);
        (*$err).error = sys::GDNativeCallErrorType_GDNATIVE_CALL_OK;
    };
}
