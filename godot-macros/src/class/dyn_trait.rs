/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// note - it is WiP, so forgive any weird docs, comments and whatnot

use crate::class::{ReceiverType, SignatureInfo};
use crate::util::{bail, ident, KvParser};
use crate::{util, ParseResult};
use proc_macro2::TokenStream;
use proc_macro2::{Ident, TokenTree};
use quote::{quote, ToTokens};

enum DynTraitMethod {
    /// Methods that are explicitly marked as non-dispatchable by user.
    NonDispatchable(SignatureInfo),
    /// Allegedly [object-safe](https://doc.rust-lang.org/reference/items/traits.html#object-safety) methods. Making sure that they are for sure object safe is user responsibility.
    Dispatchable(SignatureInfo),
}

struct DynTraitDispatch {
    /// Base godot type used by our wrapper.
    base_ty: Ident,
    trait_name: Ident,
    /// Name of our internal dispatch method
    dispatch_name: Ident,
    /// dynTrait object name.
    wrapper_name: Ident,
    /// Signatures for our normal methods
    methods_signatures: Vec<DynTraitMethod>,
}

/// Creates non-dispatchable method for our wrapper.
/// Generated method on wrapper panics on use and informs user why they can't dynamically dispatch a method explicitly marked as non-dispatchable
fn create_non_dispatchable_method(signature: SignatureInfo) -> TokenStream {
    let method_name = signature.method_name;
    let ret = signature.ret_type;
    let function_params: Vec<TokenStream> = signature
        .param_types
        .iter()
        .zip(signature.param_idents.iter())
        .map(|(ident, param)| quote! {#param: #ident})
        .collect();
    let receiver = match signature.receiver_type {
        ReceiverType::Ref => quote! {&self},
        ReceiverType::Mut => quote! {&mut self},
        ReceiverType::Static => TokenStream::default(),
        _ => unreachable!(),
    };
    let panic_message = format!(
        "error: the {} method cannot be invoked on a trait object",
        method_name
    );
    quote! {
        fn #method_name(#receiver #(#function_params),*) -> #ret {
            panic!(#panic_message)
        }
    }
}

/// Codegen for methods on our wrapper and dispatched closures
fn create_dispatch_method(
    signature: SignatureInfo,
    base_ty: &Ident,
    trait_name: &Ident,
) -> ParseResult<(TokenStream, TokenStream, TokenStream)> {
    let dispatch_func_name = ident(&format!("dispatch_{}", signature.method_name));
    let ret = signature.ret_type;
    let param_types = signature.param_types;
    let param_idents = signature.param_idents;
    let method_name = signature.method_name;
    let function_params: Vec<TokenStream> = param_types
        .iter()
        .zip(param_idents.iter())
        .map(|(ident, param)| quote! {#param: #ident})
        .collect();
    let (mutability, receiver, bind) = match signature.receiver_type {
        ReceiverType::Ref => (
            quote! { & },
            quote! {&self, },
            quote! {
                let instance = base.cast::<T>();
                let guard: GdRef<T> = instance.bind();
            },
        ),
        ReceiverType::Mut => (
            quote! { &mut },
            quote! {&mut self, },
            quote! {
                let mut instance = base.cast::<T>();
                let mut guard: GdMut<T> = instance.bind_mut();
            },
        ),
        _ => {
            // return proper error to user.
            return bail!(
                &method_name,
        "error[E0038]: the trait cannot be made into an object.\n \
        note: for a trait to be \"object safe\" it needs to allow \
        building a vtable to allow the call to be resolvable dynamically; \
        for more information visit <https://doc.rust-lang.org/reference/items/traits.html#object-safety>"
            );
        }
    };
    let fields = quote! {#dispatch_func_name: fn(Gd<#base_ty>, #(#param_types,)* fn(#mutability dyn #trait_name, #(#param_types),*) -> #ret) -> #ret};
    let declarations = quote! {
        #dispatch_func_name: |base, #(#param_idents),*, closure| {
                #bind
                closure(#mutability *guard, #(#param_idents),*)
            }
    };
    let methods = quote! {
        fn #method_name(#receiver #(#function_params),*) -> #ret {
            unsafe {((*self.dispatch).#dispatch_func_name)(self.base.clone(), #(#param_idents,)* |dispatch: #mutability dyn #trait_name, #(#param_idents,)*| {dispatch.#method_name(#(#param_idents),*)})}
        }
    };

    Ok((fields, declarations, methods))
}

/// Naive check for where Self: Sized
fn is_function_sized(f: &mut venial::Function) -> bool {
    if let Some(where_clause) = f.where_clause.as_ref() {
        return where_clause.items.items().any(|i| {
            let Some(TokenTree::Ident(bound)) = &i
                .bound
                .tokens
                .iter()
                .find(|x| matches!(x, TokenTree::Ident(_)))
            else {
                return false;
            };
            if *bound != "Sized" {
                return false;
            }
            let left_side = i
                .left_side
                .iter()
                .fold(String::new(), |acc, arg| format!("{acc}{arg}"));
            left_side == "Self"
        });
    };
    false
}

/// Checks if given associated function is explicitly non-dispatchable
/// Explicitly non-dispatchable functions have a `where Self: Sized` bound.
fn check_if_dispatchable(
    f: &mut venial::Function,
    signature_info: SignatureInfo,
) -> DynTraitMethod {
    if is_function_sized(f) {
        // This trait is object-safe, but this method can't be dispatched on a trait object.
        return DynTraitMethod::NonDispatchable(signature_info);
    }
    DynTraitMethod::Dispatchable(signature_info)
}

/// Creates signature for given function and check if it is marked as non-dispatchable.
fn parse_associated_function(f: &mut venial::Function) -> DynTraitMethod {
    let mut receiver_type: ReceiverType = ReceiverType::Static;
    let signature = util::reduce_to_signature(f);
    let num_params = signature.params.len();
    let mut param_idents = Vec::with_capacity(num_params);
    let mut param_types = Vec::with_capacity(num_params);

    let ret_type = match signature.return_ty {
        None => quote! { () },
        Some(ty) => ty.to_token_stream(),
    };

    for (arg, _) in signature.params.inner {
        match arg {
            venial::FnParam::Receiver(recv) => {
                receiver_type = if recv.tk_mut.is_some() {
                    ReceiverType::Mut
                } else if recv.tk_ref.is_some() {
                    ReceiverType::Ref
                } else {
                    // Receiver is not present at all.
                    unreachable!()
                };
            }
            venial::FnParam::Typed(arg) => {
                let ty = venial::TypeExpr {
                    tokens: arg.ty.tokens,
                };

                param_types.push(ty);
                param_idents.push(arg.name);
            }
        }
    }

    // we need to provide correct signature for given trait method
    // even if it is not dispatchable.
    let signature = SignatureInfo {
        method_name: signature.name,
        receiver_type,
        param_idents,
        param_types,
        ret_type,
    };
    check_if_dispatchable(f, signature)
}

pub fn attribute_dyn_trait(input_decl: venial::Item) -> ParseResult<TokenStream> {
    let venial::Item::Trait(mut decl) = input_decl.clone() else {
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

    // #[dyn_trait]
    if let Some(mut parser) = KvParser::parse(&attr, "dyn_trait")? {
        if let Ok(Some(name)) = parser.handle_ident("name") {
            wrapper_name = name;
        };
        if let Ok(Some(base)) = parser.handle_ident("base") {
            base_ty = base;
        };
    }

    let mut trait_dispatch = DynTraitDispatch {
        base_ty,
        trait_name,
        dispatch_name,
        wrapper_name,
        methods_signatures: vec![],
    };

    for trait_member in decl.body_items.iter_mut() {
        let venial::TraitMember::AssocFunction(f) = trait_member else {
            bail!(
                trait_member,
                "error[E0038]: the trait cannot be made into an object\n\
                note: for a trait to be \"object safe\" it needs to allow building a vtable to allow the call to be resolvable dynamically; \
                for more information visit <https://doc.rust-lang.org/reference/items/traits.html#object-safety>"
            )?
        };
        trait_dispatch
            .methods_signatures
            .push(parse_associated_function(f));
    }
    create_dyn_trait_dispatch(trait_dispatch, decl.to_token_stream())
}

/// Codegen for `#[dyn_trait] for trait MyTrait`
fn create_dyn_trait_dispatch(
    s: DynTraitDispatch,
    initial: TokenStream,
) -> ParseResult<TokenStream> {
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
        match signature {
            DynTraitMethod::NonDispatchable(sig) => {
                wrapper_methods.push(create_non_dispatchable_method(sig));
            }
            DynTraitMethod::Dispatchable(sig) => {
                let (fields, declarations, methods) =
                    create_dispatch_method(sig, &base_ty, &trait_name)?;
                dispatch_fields.push(fields);
                dispatch_declarations.push(declarations);
                wrapper_methods.push(methods);
            }
        }
    }

    let ret = quote! {
        #initial

        static #registry_name: godot::sys::Global<std::collections::HashMap<String, #dispatch_name>> = godot::sys::Global::default();

        pub fn #register_dispatch_name<T>()
            where
                T: Inherits<#base_ty> + GodotClass + godot::obj::Bounds<Declarer = godot::obj::bounds::DeclUser> + #trait_name
        {
            let name = T::class_name().to_string();
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
            pub fn new(base: Gd<#base_ty>) -> Result<Self, &'static str> {
                let registry = #registry_name.lock();
                let Some(dispatch) = registry.get(&base.get_class().to_string()) else {
                    return Err("Given class is not registered as dynTrait object!")
                };
                Ok(Self {
                    base,
                    dispatch: dispatch as *const #dispatch_name
                })
            }
        }

        impl #trait_name for #wrapper_name {
            #(#wrapper_methods)*
        }

        impl GodotConvert for #wrapper_name {
            type Via = Gd<#base_ty>;
        }

        impl ToGodot for #wrapper_name {
            type ToVia<'v> = Gd<#base_ty>
                where Self: 'v;
            fn to_godot(&self) -> Self::ToVia<'_> {
                self.base.clone()
            }
        }

        impl FromGodot for #wrapper_name {
            fn try_from_godot(via: Self::Via) -> Result<Self, ConvertError> {
                match #wrapper_name::new(via) {
                    Ok(s) => Ok(s),
                    Err(message) => Err(ConvertError::new(message))
                }
            }
        }

    };
    Ok(ret)
}
