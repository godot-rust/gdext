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
    let sig = get_sig(&signature_info.ret_type, &signature_info.param_types);

    let method_name = &signature_info.method_name;
    let param_idents = &signature_info.param_idents;

    let method_flags = method_flags(signature_info.receiver_type);

    let wrapped_method = wrap_with_unpacked_params(class_name, &signature_info);

    let varcall_func = get_varcall_func(class_name, method_name, &sig, &wrapped_method);
    let ptrcall_func = get_ptrcall_func(class_name, method_name, &sig, &wrapped_method);

    quote! {
        {
            use godot::builtin::meta::*;
            use godot::builtin::{StringName, Variant};
            use godot::sys;

            let class_name = ClassName::from_static(stringify!(#class_name));
            let method_name = StringName::from(stringify!(#method_name));

            type Sig = #sig;

            let varcall_func = #varcall_func;
            let ptrcall_func = #ptrcall_func;

            let method_info = MethodInfo::from_signature::<Sig>(
                class_name,
                method_name,
                Some(varcall_func),
                Some(ptrcall_func),
                #method_flags,
                &[
                    #( stringify!(#param_idents) ),*
                ],
                Vec::new()
            );

            godot::private::out!(
                "   Register fn:   {}::{}",
                stringify!(#class_name),
                stringify!(#method_name)
            );

            method_info.register_extension_class_method();
        };
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
