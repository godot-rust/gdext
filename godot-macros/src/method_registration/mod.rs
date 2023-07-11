/*
* This Source Code Form is subject to the terms of the Mozilla Public
* License, v. 2.0. If a copy of the MPL was not distributed with this
* file, You can obtain one at https://mozilla.org/MPL/2.0/.
*/
mod register_method;
mod virtual_method_callback;

use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};

pub use register_method::make_method_registration;
pub use virtual_method_callback::gdext_virtual_method_callback;

#[derive(Copy, Clone, Eq, PartialEq)]
enum ReceiverType {
    Ref,
    Mut,
    Static,
}

struct SignatureInfo {
    pub method_name: Ident,
    pub receiver_type: ReceiverType,
    pub param_idents: Vec<Ident>,
    pub param_types: Vec<venial::TyExpr>,
    pub ret_type: TokenStream,
}

/// Returns a closure expression that forwards the parameters to the Rust instance.
fn make_forwarding_closure(class_name: &Ident, signature_info: &SignatureInfo) -> TokenStream {
    let method_name = &signature_info.method_name;
    let params = &signature_info.param_idents;

    let instance_decl = match &signature_info.receiver_type {
        ReceiverType::Ref => quote! {
            let instance = storage.get();
        },
        ReceiverType::Mut => quote! {
            let mut instance = storage.get_mut();
        },
        _ => quote! {},
    };

    match signature_info.receiver_type {
        ReceiverType::Ref | ReceiverType::Mut => {
            quote! {
                |instance_ptr, params| {
                    let ( #(#params,)* ) = params;

                    let storage =
                        unsafe { godot::private::as_storage::<#class_name>(instance_ptr) };
                    #instance_decl

                    instance.#method_name(#(#params),*)
                }
            }
        }
        ReceiverType::Static => {
            quote! {
                |_, params| {
                    let ( #(#params,)* ) = params;
                    <#class_name>::#method_name(#(#params),*)
                }
            }
        }
    }
}

fn get_signature_info(signature: &venial::Function) -> SignatureInfo {
    let method_name = signature.name.clone();
    let mut receiver_type = ReceiverType::Static;
    let mut param_idents: Vec<Ident> = Vec::new();
    let mut param_types = Vec::new();
    let ret_type = match &signature.return_ty {
        None => quote! { () },
        Some(ty) => quote! { #ty },
    };

    let mut next_unnamed_index = 0;
    for (arg, _) in &signature.params.inner {
        match arg {
            venial::FnParam::Receiver(recv) => {
                receiver_type = if recv.tk_mut.is_some() {
                    ReceiverType::Mut
                } else if recv.tk_ref.is_some() {
                    ReceiverType::Ref
                } else {
                    panic!("Receiver not supported");
                };
            }
            venial::FnParam::Typed(arg) => {
                // Parameter will be forwarded as an argument to the instance, so we need to give `_` a name.
                let ident = if arg.name == "_" {
                    let ident = format_ident!("__unnamed_{next_unnamed_index}");
                    next_unnamed_index += 1;
                    ident
                } else {
                    arg.name.clone()
                };
                let ty = arg.ty.clone();

                param_types.push(ty);
                param_idents.push(ident);
            }
        }
    }

    SignatureInfo {
        method_name,
        receiver_type,
        param_idents,
        param_types,
        ret_type,
    }
}

fn make_method_flags(method_type: ReceiverType) -> TokenStream {
    match method_type {
        ReceiverType::Ref | ReceiverType::Mut => {
            quote! { ::godot::engine::global::MethodFlags::METHOD_FLAGS_DEFAULT }
        }
        ReceiverType::Static => {
            quote! { ::godot::engine::global::MethodFlags::METHOD_FLAG_STATIC }
        }
    }
}

/// Generate code for a `ptrcall` call expression.
fn make_ptrcall_invocation(
    method_name: &Ident,
    sig_tuple: &TokenStream,
    wrapped_method: &TokenStream,
    is_virtual: bool,
) -> TokenStream {
    let method_name_str = method_name.to_string();

    let ptrcall_type = if is_virtual {
        quote! { sys::PtrcallType::Virtual }
    } else {
        quote! { sys::PtrcallType::Standard }
    };

    quote! {
         <#sig_tuple as godot::builtin::meta::PtrcallSignatureTuple>::ptrcall(
            instance_ptr,
            args,
            ret,
            #wrapped_method,
            #method_name_str,
            #ptrcall_type,
        )
    }
}

/// Generate code for a `varcall()` call expression.
fn make_varcall_invocation(
    method_name: &Ident,
    sig_tuple: &TokenStream,
    wrapped_method: &TokenStream,
) -> TokenStream {
    let method_name_str = method_name.to_string();

    quote! {
        <#sig_tuple as godot::builtin::meta::VarcallSignatureTuple>::varcall(
            instance_ptr,
            args,
            ret,
            err,
            #wrapped_method,
            #method_name_str,
        )
    }
}
