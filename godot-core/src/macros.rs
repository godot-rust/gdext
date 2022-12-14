/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

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
            use $crate::sys;
            use $crate::builtin::{Variant, StringName};
            use $crate::builtin::meta::*;

            const NUM_ARGS: usize = $crate::gdext_count_idents!($( $param, )*);

            type Sig = ( $($RetTy)+, $($ParamTy),* );

            let varcall_func = {
                unsafe extern "C" fn function(
                    _method_data: *mut std::ffi::c_void,
                    instance_ptr: sys::GDExtensionClassInstancePtr,
                    args: *const sys::GDExtensionConstVariantPtr,
                    _arg_count: sys::GDExtensionInt,
                    ret: sys::GDExtensionVariantPtr,
                    err: *mut sys::GDExtensionCallError,
                ) {
                    let result = ::std::panic::catch_unwind(|| {
                        <Sig as SignatureTuple>::varcall::< $Class >(
                            instance_ptr,
                            args,
                            ret,
                            err,
                            |inst, params| {
                                let ( $($param,)* ) = params;
                                inst.$method_name( $( $param, )* )
                            },
                            stringify!($method_name),
                        )
                    });

                    if let Err(e) = result {
                        $crate::log::godot_error!("Rust function panicked: {}", stringify!($method_name));
                        $crate::private::print_panic(e);

                        // Signal error and set return type to Nil
                        (*err).error = sys::GDEXTENSION_CALL_ERROR_INVALID_METHOD; // no better fitting enum?
                        sys::interface_fn!(variant_new_nil)(ret);
                    }
                }

                function
            };

            let ptrcall_func = {
                unsafe extern "C" fn function(
                    _method_data: *mut std::ffi::c_void,
                    instance_ptr: sys::GDExtensionClassInstancePtr,
                    args: *const sys::GDExtensionConstTypePtr,
                    ret: sys::GDExtensionTypePtr,
                ) {
                    let result = ::std::panic::catch_unwind(|| {
                        <Sig as SignatureTuple>::ptrcall::< $Class >(
                            instance_ptr,
                            args,
                            ret,
                            |inst, params| {
                                let ( $($param,)* ) = params;
                                inst.$method_name( $( $param, )* )
                            },
                            stringify!($method_name),
                        );
                    });

                    if let Err(e) = result {
                        $crate::log::godot_error!("Rust function panicked: {}", stringify!($method_name));
                        $crate::private::print_panic(e);

                        // TODO set return value to T::default()?
                    }
                }

                function
            };

            // Return value meta-information
            let has_return_value: bool = $crate::gdext_is_not_unit!($($RetTy)+);
            let return_value_info = Sig::property_info(-1, "");
            let mut return_value_info_sys = return_value_info.property_sys();
            let return_value_metadata = Sig::param_metadata(-1);

            // Arguments meta-information
            let argument_count = NUM_ARGS as u32;
            let mut arguments_info: [PropertyInfo; NUM_ARGS] = {
                let mut i = -1i32;
                [$(
                    {
                        i += 1;
                        let prop = Sig::property_info(i, stringify!($param));
                        //OnceArg::new(prop)
                        prop
                    },
                )*]
            };
            let mut arguments_info_sys: [sys::GDExtensionPropertyInfo; NUM_ARGS]
                = std::array::from_fn(|i| arguments_info[i].property_sys());
//                = std::array::from_fn(|i| arguments_info[i].once_sys());
            let mut arguments_metadata: [sys::GDExtensionClassMethodArgumentMetadata; NUM_ARGS]
                = std::array::from_fn(|i| Sig::param_metadata(i as i32));

            let class_name = StringName::from(stringify!($Class));
            let method_name = StringName::from(stringify!($method_name));

            // println!("REG {class_name}::{method_name}");
            // println!("  ret {return_value_info:?}");

            let method_info = sys::GDExtensionClassMethodInfo {
                name: method_name.string_sys(),
                method_userdata: std::ptr::null_mut(),
                call_func: Some(varcall_func),
                ptrcall_func: Some(ptrcall_func),
                method_flags: sys::GDEXTENSION_METHOD_FLAGS_DEFAULT as u32,
                has_return_value: has_return_value as u8,
                return_value_info: std::ptr::addr_of_mut!(return_value_info_sys),
                return_value_metadata,
                argument_count,
                arguments_info: arguments_info_sys.as_mut_ptr(),
                arguments_metadata: arguments_metadata.as_mut_ptr(),
                default_argument_count: 0,
                default_arguments: std::ptr::null_mut(),
            };

            $crate::out!("   Register fn:   {}::{}", stringify!($Class), stringify!($method_name));
            sys::interface_fn!(classdb_register_extension_class_method)(
                sys::get_library(),
                class_name.string_sys(),
                std::ptr::addr_of!(method_info),
            );

            // std::mem::forget(class_name);
            // std::mem::forget(method_name);
        }
    };
}

/// Convenience macro to wrap an obj's method into a function pointer
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
        ) -> $RetTy:ty
    ) => {
        $crate::gdext_register_method_inner!(
            $Class,
            map_mut,
            fn $method_name(
                $( $param : $ParamTy, )*
                $( #[opt] $opt_param : $OptParamTy, )*
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
        ) -> $RetTy:ty
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
        )
    ) => {
        // recurse this macro
        $crate::gdext_register_method!(
            $Class,
            fn $method_name(
                &mut self,
                $( $param : $ParamTy, )*
                $( #[opt] $opt_param : $OptParamTy, )*
            ) -> ()
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
        )
    ) => {
        // recurse this macro
        $crate::gdext_register_method!(
            $Class,
            fn $method_name(
                &self,
                $( $param : $ParamTy, )*
                $( #[opt] $opt_param : $OptParamTy, )*
            ) -> ()
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
            use $crate::sys;

            unsafe extern "C" fn function(
                instance_ptr: sys::GDExtensionClassInstancePtr,
                args: *const sys::GDExtensionConstTypePtr,
                ret: sys::GDExtensionTypePtr,
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
        use $crate::sys;

        $crate::out!("ptrcall: {}", stringify!($method_name));
        let storage = $crate::private::as_storage::<$Class>($instance_ptr);
        let mut instance = storage.get_mut();

        let mut idx = 0;
        $(
            let $arg = <$ParamTy as sys::GodotFfi>::from_sys(sys::force_mut_ptr(*$args.offset(idx)));
            // FIXME update refcount, e.g. Gd::ready() or T::Mem::maybe_inc_ref(&result);
            // possibly in from_sys() directly; what about from_sys_init() and from_{obj|str}_sys()?
            idx += 1;
        )*

        let ret_val = instance.$method_name($(
            $arg,
        )*);

        <$($RetTy)+ as sys::GodotFfi>::write_sys(&ret_val, $ret);
        // FIXME should be inc_ref instead of forget
        std::mem::forget(ret_val);
    };
}
