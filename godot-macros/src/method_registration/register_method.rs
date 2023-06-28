/*
* This Source Code Form is subject to the terms of the Mozilla Public
* License, v. 2.0. If a copy of the MPL was not distributed with this
* file, You can obtain one at https://mozilla.org/MPL/2.0/.
*/

use crate::method_registration::{
    get_sig, get_signature_info, method_flags, wrap_with_unpacked_params,
};
use crate::util::reduce_to_signature;
use proc_macro2::{Ident, TokenStream};
use quote::quote;
use venial::parse_declaration;

use super::SignatureInfo;

// Convenience function to wrap an object's method into a function pointer
// that can be passed to the engine when registering a class.
pub fn gdext_register_method(class_name: &Ident, method_signature: &TokenStream) -> TokenStream {
    let method_declaration = parse_declaration(quote! { #method_signature {}})
        .unwrap()
        .as_function()
        .unwrap()
        .clone();
    let method_signature = reduce_to_signature(&method_declaration);
    let signature_info = get_signature_info(&method_signature);
    let method_name = &signature_info.method_name;

    let sig = get_sig(&signature_info.ret_type, &signature_info.param_types);
    let method_info = get_method_info(class_name, &signature_info, sig);

    quote! {
        {
            use godot::builtin::meta::*;
            use godot::builtin::{StringName, Variant};
            use godot::sys;

            let class_name = StringName::from(stringify!(#class_name));
            let method_name = StringName::from(stringify!(#method_name));

            let method_info = #method_info;

            godot::private::out!(
                "   Register fn:   {}::{}",
                stringify!(#class_name),
                stringify!(#method_name)
            );

            unsafe {
                sys::interface_fn!(classdb_register_extension_class_method)(
                    sys::get_library(),
                    class_name.string_sys(),
                    std::ptr::addr_of!(method_info),
                );
            }
        };
    }
}

fn get_method_info(
    class_name: &Ident,
    signature_info: &SignatureInfo,
    sig: TokenStream,
) -> TokenStream {
    let method_name = &signature_info.method_name;
    let has_return_value = signature_info.has_return_value;
    let param_idents = &signature_info.param_idents;
    let num_args = signature_info.num_args;

    let method_flags = method_flags(signature_info.receiver_type);

    let wrapped_method = wrap_with_unpacked_params(class_name, signature_info);

    let varcall_func = get_varcall_func(class_name, method_name, &sig, &wrapped_method);
    let ptrcall_func = get_ptrcall_func(class_name, method_name, &sig, &wrapped_method);

    quote! {
        {
            type Sig = #sig;

            let varcall_func = #varcall_func;
            let ptrcall_func = #ptrcall_func;

            // Return value meta-information
            let has_return_value: bool = #has_return_value;
            let return_value_info = Sig::property_info(-1, "");
            let mut return_value_info_sys = return_value_info.property_sys();
            let return_value_metadata = Sig::param_metadata(-1);

            // Arguments meta-information
            let argument_count = #num_args as u32;

            // We dont want to drop `arguments_info` before we're done with using `arguments_info_sys`.
            let arguments_info: [PropertyInfo; #num_args] =
                godot::private::gdext_get_arguments_info!(Sig, #( #param_idents, )*);

            let mut arguments_info_sys: [sys::GDExtensionPropertyInfo; #num_args] =
                std::array::from_fn(|i| arguments_info[i].property_sys());
            let mut arguments_metadata: [sys::GDExtensionClassMethodArgumentMetadata;
                #num_args] = std::array::from_fn(|i| Sig::param_metadata(i as i32));

            let method_info = sys::GDExtensionClassMethodInfo {
                name: method_name.string_sys(),
                method_userdata: std::ptr::null_mut(),
                call_func: Some(varcall_func),
                ptrcall_func: Some(ptrcall_func),
                method_flags: #method_flags as u32,
                has_return_value: has_return_value as u8,
                return_value_info: std::ptr::addr_of_mut!(return_value_info_sys),
                return_value_metadata,
                argument_count,
                arguments_info: arguments_info_sys.as_mut_ptr(),
                arguments_metadata: arguments_metadata.as_mut_ptr(),
                default_argument_count: 0,
                default_arguments: std::ptr::null_mut(),
            };

            method_info
        }
    }
}

fn get_varcall_func(
    class_name: &Ident,
    method_name: &Ident,
    sig: &TokenStream,
    wrapped_method: &TokenStream,
) -> TokenStream {
    quote! {
        {
            unsafe extern "C" fn function(
                _method_data: *mut std::ffi::c_void,
                instance_ptr: sys::GDExtensionClassInstancePtr,
                args: *const sys::GDExtensionConstVariantPtr,
                _arg_count: sys::GDExtensionInt,
                ret: sys::GDExtensionVariantPtr,
                err: *mut sys::GDExtensionCallError,
            ) {
                let success = godot::private::handle_panic(
                    || stringify!(#method_name),
                    || {
                        godot::private::gdext_call_signature_method!(
                            varcall,
                            #sig,
                            #class_name,
                            instance_ptr,
                            args,
                            ret,
                            err,
                            #wrapped_method,
                            #method_name
                        );
                    },
                );

                if success.is_none() {
                    // Signal error and set return type to Nil
                    (*err).error = sys::GDEXTENSION_CALL_ERROR_INVALID_METHOD; // no better fitting enum?

                    // TODO(uninit)
                    sys::interface_fn!(variant_new_nil)(sys::AsUninit::as_uninit(ret));
                }
            }

            function
        }
    }
}

fn get_ptrcall_func(
    class_name: &Ident,
    method_name: &Ident,
    sig: &TokenStream,
    wrapped_method: &TokenStream,
) -> TokenStream {
    quote! {
        {
            unsafe extern "C" fn function(
                _method_data: *mut std::ffi::c_void,
                instance_ptr: sys::GDExtensionClassInstancePtr,
                args: *const sys::GDExtensionConstTypePtr,
                ret: sys::GDExtensionTypePtr,
            ) {
                let success = godot::private::handle_panic(
                    || stringify!(#method_name),
                    || {
                        godot::private::gdext_call_signature_method!(
                            ptrcall,
                            #sig,
                            #class_name,
                            instance_ptr,
                            args,
                            ret,
                            #wrapped_method,
                            #method_name,
                            sys::PtrcallType::Standard
                        );
                    },
                );

                if success.is_none() {
                    // TODO set return value to T::default()?
                }
            }

            function
        }
    }
}
