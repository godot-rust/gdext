/*
* This Source Code Form is subject to the terms of the Mozilla Public
* License, v. 2.0. If a copy of the MPL was not distributed with this
* file, You can obtain one at https://mozilla.org/MPL/2.0/.
*/
mod register_method;
mod virtual_method_callback;

use proc_macro2::{Ident, TokenStream};
use quote::quote;
use venial::{Function, TyExpr};

pub use register_method::gdext_register_method;
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
    pub param_types: Vec<TyExpr>,
    pub ret_type: TokenStream,
}

fn wrap_with_unpacked_params(class_name: &Ident, signature_info: &SignatureInfo) -> TokenStream {
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

fn get_signature_info(signature: &Function) -> SignatureInfo {
    let method_name = signature.name.clone();
    let mut receiver_type = ReceiverType::Static;
    let mut param_idents: Vec<Ident> = Vec::new();
    let mut param_types = Vec::new();
    let ret_type = match &signature.return_ty {
        None => quote! { () },
        Some(ty) => quote! { #ty },
    };

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
                let ident = arg.name.clone();
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

fn method_flags(method_type: ReceiverType) -> TokenStream {
    match method_type {
        ReceiverType::Ref | ReceiverType::Mut => {
            quote! { ::godot::engine::global::MethodFlags::METHOD_FLAGS_DEFAULT }
        }
        ReceiverType::Static => {
            quote! { ::godot::engine::global::MethodFlags::METHOD_FLAG_STATIC }
        }
    }
}

fn get_sig(ret_type: &TokenStream, param_types: &Vec<TyExpr>) -> TokenStream {
    quote! {
        (#ret_type, #(#param_types),*)
    }
}
