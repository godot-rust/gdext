/*
* This Source Code Form is subject to the terms of the Mozilla Public
* License, v. 2.0. If a copy of the MPL was not distributed with this
* file, You can obtain one at https://mozilla.org/MPL/2.0/.
*/

use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::quote;
use venial::Function;

use crate::method_registration::{get_sig, get_signature_info, wrap_with_unpacked_params};

/// Returns a C function which acts as the callback when a virtual method of this instance is invoked.
//
// There are currently no virtual static methods. Additionally, virtual static methods dont really make a lot
// of sense. Therefore there is no need to support them.
pub fn gdext_virtual_method_callback(
    class_name: &Ident,
    method_signature: &Function,
) -> TokenStream2 {
    let signature_info = get_signature_info(method_signature);
    let method_name = &method_signature.name;

    let wrapped_method = wrap_with_unpacked_params(class_name, &signature_info);
    let sig = get_sig(&signature_info.ret_type, &signature_info.param_types);

    quote! {
        {
            use godot::sys;

            unsafe extern "C" fn function(
                instance_ptr: sys::GDExtensionClassInstancePtr,
                args: *const sys::GDExtensionConstTypePtr,
                ret: sys::GDExtensionTypePtr,
            ) {
                godot::private::gdext_call_signature_method!(
                    ptrcall,
                    #sig,
                    instance_ptr,
                    args,
                    ret,
                    #wrapped_method,
                    #method_name,
                    godot::sys::PtrcallType::Virtual
                );
            }
            Some(function)
        }
    }
}
