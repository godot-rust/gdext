/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::class::{ReceiverType, SignatureInfo};
use crate::util::{bail, ident, KvParser};
use crate::{util, ParseResult};
use proc_macro2::TokenStream;
use proc_macro2::{Group, Ident, TokenTree};
use quote::{quote, ToTokens};
use venial::TraitMember;

struct DynTraitDispatch {
    base_ty: Ident,
    trait_name: Ident,
    dispatch_name: Ident,
    wrapper_name: Ident,
    methods_signatures: Vec<SignatureInfo>,
}

fn parse_trait_member(params: &mut TraitMember, method_signatures: &mut Vec<SignatureInfo>) {
    match params {
        TraitMember::AssocFunction(f) => {
            let mut receiver_type: Option<ReceiverType> = None;
            let signature = util::reduce_to_signature(f);
            let num_params = signature.params.len();
            let mut param_idents = Vec::with_capacity(num_params);
            let mut param_types = Vec::with_capacity(num_params);
            let ret_type = match signature.return_ty {
                None => quote! { () },
                Some(ty) => ty
                    .tokens
                    .into_iter()
                    .map(|tt| match tt {
                        TokenTree::Group(group) => {
                            TokenTree::Group(Group::new(group.delimiter(), group.stream()))
                        }
                        tt => tt,
                    })
                    .collect(),
            };

            for (arg, _) in signature.params.inner {
                match arg {
                    venial::FnParam::Receiver(recv) => {
                        receiver_type = if recv.tk_mut.is_some() {
                            Some(ReceiverType::Mut)
                        } else if recv.tk_ref.is_some() {
                            Some(ReceiverType::Ref)
                        } else {
                            panic!("Receiver not supported");
                        };
                    }
                    venial::FnParam::Typed(arg) => {
                        let ty = venial::TypeExpr {
                            tokens: arg
                                .ty
                                .tokens
                                .into_iter()
                                .map(|tt| match tt {
                                    TokenTree::Group(group) => TokenTree::Group(Group::new(
                                        group.delimiter(),
                                        group.stream(),
                                    )),
                                    tt => tt,
                                })
                                .collect(),
                        };
                        param_types.push(ty);
                        param_idents.push(arg.name);
                    }
                }
            }

            method_signatures.push(SignatureInfo {
                method_name: signature.name,
                receiver_type: receiver_type.unwrap_or(ReceiverType::Static),
                param_idents,
                param_types,
                ret_type,
            });
        }
        TraitMember::AssocConstant(c) => {
            unimplemented!()
        }
        TraitMember::AssocType(t) => {
            unimplemented!()
        }
        TraitMember::Macro(_) => unimplemented!(),
        _ => {}
    }
}

pub fn attribute_dyn_trait(input_decl: venial::Item) -> ParseResult<TokenStream> {
    let venial::Item::Trait(mut decl) = input_decl else {
        bail!(
            input_decl,
            "#[dyn_trait] can only be applied on trait blocks",
        )?
    };
    let trait_name = decl.name.clone();
    let dispatch_name = ident(&format! {"{}GdDispatch", decl.name});
    let mut wrapper_name = ident(&format!("{}GdDyn", decl.name));
    let mut base_ty = ident("Object");
    let attr = std::mem::take(&mut decl.attributes);
    if let Some(mut parser) = KvParser::parse(&attr, "dyn_trait")? {
        if let Ok(Some(name)) = parser.handle_ident("name") {
            wrapper_name = name;
        };
        if let Ok(Some(base)) = parser.handle_ident("base") {
            base_ty = base;
        };
    }

    let mut methods_signatures = Vec::new();
    for trait_member in decl.body_items.iter_mut() {
        parse_trait_member(trait_member, &mut methods_signatures);
    }
    let trait_dispatch = DynTraitDispatch {
        base_ty,
        trait_name,
        dispatch_name,
        wrapper_name,
        methods_signatures,
    };
    let ret = create_dyn_trait_dispatch(trait_dispatch, decl.to_token_stream());
    eprintln!("{ret}");
    ParseResult::Ok(ret)
}

pub fn create_dyn_trait_dispatch(s: DynTraitDispatch, initial: TokenStream) -> TokenStream {
    let DynTraitDispatch {
        base_ty,
        trait_name,
        dispatch_name,
        wrapper_name,
        methods_signatures,
    } = s;
    let registry_name = ident(&format!(
        "{}_DISPATCH_REGISTRY",
        trait_name.to_string().to_uppercase()
    ));
    let register_dispatch_name = ident(&format!(
        "register_{}_dispatch",
        trait_name.to_string().to_lowercase()
    ));
    let mut dispatch_fields: Vec<TokenStream> = vec![];
    let mut dispatch_declarations: Vec<TokenStream> = vec![];
    let mut wrapper_methods: Vec<TokenStream> = vec![];
    for signature in methods_signatures.into_iter() {
        let dispatch_func_name = ident(&format!("dispatch_{}", signature.method_name));
        let (mutability, receiver) = match signature.receiver_type {
            ReceiverType::Ref => (quote! { & }, quote! {&self}),
            ReceiverType::Mut => (quote! { &mut }, quote! {&mut self}),
            ReceiverType::GdSelf => !unreachable!(),
            ReceiverType::Static => (quote! { () }, quote! {}),
        };
        let ret = signature.ret_type;
        let param_types = signature.param_types;
        let param_idents = signature.param_idents;
        let method_name = signature.method_name;
        let function_params: Vec<TokenStream> = param_types
            .iter()
            .zip(param_idents.iter())
            .map(|(ident, param)| quote! {#param: #ident})
            .collect();
        dispatch_fields.push(
            quote! {
                #dispatch_func_name: fn(Gd<#base_ty>, #(#param_types,)* fn(#mutability dyn #trait_name, #(#param_types),*) -> #ret) -> #ret
            }
        );
        dispatch_declarations.push(quote! {
            #dispatch_func_name: |base, #(#param_idents),*, closure| {
                let mut instance = base.cast::<T>();
                let mut guard: GdMut<T> = instance.bind_mut();
                closure(&mut *guard, #(#param_idents),*)
            }
        });
        wrapper_methods.push(
            quote! {
                fn #method_name(#receiver, #(#function_params),*) -> #ret {
                    unsafe {((*self.dispatch).#dispatch_func_name)(self.base.clone(), #(#param_idents,)* |dispatch: #mutability dyn #trait_name, #(#param_idents,)*| {dispatch.#method_name(#(#param_idents),*)})}
                }
            }
        );
    }

    quote! {
        #initial

        static #registry_name: godot::sys::Global<std::collections::HashMap<String, #dispatch_name>> = godot::sys::Global::default();


        pub fn #register_dispatch_name<T>(name: String)
            where
                T: Inherits<#base_ty> + GodotClass + godot::obj::Bounds<Declarer = godot::obj::bounds::DeclUser> + #trait_name
        {
            let mut registry = #registry_name.lock();
            registry.entry(name).or_insert_with(
            || #dispatch_name::new::<T>()
        );
        }

        struct #dispatch_name {
            #(#dispatch_fields),*
        }


        impl #dispatch_name {
            fn new<T>() -> Self
                where
                    T: Inherits<#base_ty> + GodotClass + godot::obj::Bounds<Declarer = godot::obj::bounds::DeclUser> + #trait_name
            {
                Self {
                #(#dispatch_declarations),*
                }
            }
        }

        pub struct #wrapper_name {
            pub base: Gd<#base_ty>,
            dispatch: *const #dispatch_name
        }

        impl #wrapper_name {
            pub fn new(base: Gd<#base_ty>) -> Self {
                let registry = #registry_name.lock();
                let dispatch = &registry[&base.get_class().to_string()] as *const #dispatch_name;
                Self {
                    base,
                    dispatch
                }
            }
        }

        impl #trait_name for #wrapper_name {
            #(#wrapper_methods)*
        }
    }
}
