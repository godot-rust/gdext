/*
* This Source Code Form is subject to the terms of the Mozilla Public
* License, v. 2.0. If a copy of the MPL was not distributed with this
* file, You can obtain one at https://mozilla.org/MPL/2.0/.
*/

use crate::method_registration::{
    get_signature_info, make_forwarding_closure, make_method_flags, make_ptrcall_invocation,
    make_varcall_invocation,
};
use crate::util;
use proc_macro2::{Ident, TokenStream};
use quote::quote;

/// Generates code that registers the specified method for the given class.
pub fn make_method_registration(
    class_name: &Ident,
    method_signature: venial::Function,
) -> TokenStream {
    let signature_info = get_signature_info(&method_signature);
    let sig_tuple =
        util::make_signature_tuple_type(&signature_info.ret_type, &signature_info.param_types);

    let method_name = &signature_info.method_name;
    let param_idents = &signature_info.param_idents;

    let method_flags = make_method_flags(signature_info.receiver_type);

    let forwarding_closure = make_forwarding_closure(class_name, &signature_info);

    let varcall_func = make_varcall_func(method_name, &sig_tuple, &forwarding_closure);
    let ptrcall_func = make_ptrcall_func(method_name, &sig_tuple, &forwarding_closure);

    // String literals
    let class_name_str = class_name.to_string();
    let method_name_str = method_name.to_string();
    let param_ident_strs = param_idents.iter().map(|ident| ident.to_string());

    quote! {
        {
            use godot::builtin::meta::*;
            use godot::builtin::meta::registration::method::MethodInfo;
            use godot::builtin::{StringName, Variant};
            use godot::sys;

            type Sig = #sig_tuple;

            let class_name = ClassName::from_static(#class_name_str);
            let method_name = StringName::from(#method_name_str);

            let varcall_func = #varcall_func;
            let ptrcall_func = #ptrcall_func;

            // SAFETY:
            // `get_varcall_func` upholds all the requirements for `call_func`.
            // `get_ptrcall_func` upholds all the requirements for `ptrcall_func`
            let method_info = unsafe {
                MethodInfo::from_signature::<Sig>(
                class_name,
                method_name,
                Some(varcall_func),
                Some(ptrcall_func),
                #method_flags,
                &[
                    #( #param_ident_strs ),*
                ],
                Vec::new()
                )
            };

            godot::private::out!(
                "   Register fn:   {}::{}",
                #class_name_str,
                #method_name_str
            );


            method_info.register_extension_class_method();
        };
    }
}

/// Generate code for a C FFI function that performs a varcall.
fn make_varcall_func(
    method_name: &Ident,
    sig_tuple: &TokenStream,
    wrapped_method: &TokenStream,
) -> TokenStream {
    let invocation = make_varcall_invocation(method_name, sig_tuple, wrapped_method);

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
                    || #invocation
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

/// Generate code for a C FFI function that performs a ptrcall.
fn make_ptrcall_func(
    method_name: &Ident,
    sig_tuple: &TokenStream,
    wrapped_method: &TokenStream,
) -> TokenStream {
    let invocation = make_ptrcall_invocation(method_name, sig_tuple, wrapped_method, false);

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
                    || #invocation
                );

                if success.is_none() {
                    // TODO set return value to T::default()?
                }
            }

            function
        }
    }
}
