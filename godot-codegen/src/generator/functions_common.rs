/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::generator::default_parameters;
use crate::models::domain::{FnParam, FnQualifier, Function, RustTy};
use crate::util::safe_ident;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

pub struct FnReceiver {
    /// `&self`, `&mut self`, (none)
    pub param: TokenStream,

    /// `ptr::null_mut()`, `self.object_ptr`, `self.sys_ptr`, (none)
    pub ffi_arg: TokenStream,

    /// `Self::`, `self.`
    pub self_prefix: TokenStream,
}

impl FnReceiver {
    /// No receiver, not even static `Self`
    pub fn global_function() -> FnReceiver {
        FnReceiver {
            param: TokenStream::new(),
            ffi_arg: TokenStream::new(),
            self_prefix: TokenStream::new(),
        }
    }
}

pub struct FnCode {
    pub receiver: FnReceiver,
    pub varcall_invocation: TokenStream,
    pub ptrcall_invocation: TokenStream,
}

pub struct FnDefinition {
    pub functions: TokenStream,
    pub builders: TokenStream,
}

impl FnDefinition {
    pub fn none() -> FnDefinition {
        FnDefinition {
            functions: TokenStream::new(),
            builders: TokenStream::new(),
        }
    }

    pub fn into_functions_only(self) -> TokenStream {
        assert!(
            self.builders.is_empty(),
            "definition of this function should not have any builders"
        );

        self.functions
    }
}

pub struct FnDefinitions {
    pub functions: TokenStream,
    pub builders: TokenStream,
}

impl FnDefinitions {
    /// Combines separate code from multiple function definitions into one, split by functions and builders.
    pub fn expand(definitions: impl Iterator<Item = FnDefinition>) -> FnDefinitions {
        // Collect needed because borrowed by 2 closures
        let definitions: Vec<_> = definitions.collect();
        let functions = definitions.iter().map(|def| &def.functions);
        let structs = definitions.iter().map(|def| &def.builders);

        FnDefinitions {
            functions: quote! { #( #functions )* },
            builders: quote! { #( #structs )* },
        }
    }
}

pub fn make_function_definition(
    sig: &dyn Function,
    code: &FnCode,
    safety_doc: Option<TokenStream>,
) -> FnDefinition {
    let has_default_params = default_parameters::function_uses_default_params(sig);
    let vis = if has_default_params {
        // Public API mapped by separate function.
        // Needs to be crate-public because default-arg builder lives outside of the module.
        quote! { pub(crate) }
    } else {
        make_vis(sig.is_private())
    };

    let (maybe_unsafe, safety_doc) = if let Some(safety_doc) = safety_doc {
        (quote! { unsafe }, safety_doc)
    } else if function_uses_pointers(sig) {
        (
            quote! { unsafe },
            quote! {
                /// # Safety
                ///
                /// Godot currently does not document safety requirements on this method. Make sure you understand the underlying semantics.
            },
        )
    } else {
        (TokenStream::new(), TokenStream::new())
    };

    let [params, param_types, arg_names] = make_params_exprs(sig.params());

    let rust_function_name_str = sig.name();
    let primary_fn_name = if has_default_params {
        format_ident!("{}_full", safe_ident(rust_function_name_str))
    } else {
        safe_ident(rust_function_name_str)
    };

    let (default_fn_code, default_structs_code) = if has_default_params {
        default_parameters::make_function_definition_with_defaults(sig, code, &primary_fn_name)
    } else {
        (TokenStream::new(), TokenStream::new())
    };

    let return_ty = &sig.return_value().type_tokens();
    let call_sig = quote! {
        ( #return_ty, #(#param_types),* )
    };

    let return_decl = &sig.return_value().decl;

    let receiver_param = &code.receiver.param;
    let primary_function = if sig.is_virtual() {
        // Virtual functions

        quote! {
            #safety_doc
            #maybe_unsafe fn #primary_fn_name(
                #receiver_param
                #( #params, )*
            ) #return_decl {
                unimplemented!()
            }
        }
    } else if sig.is_vararg() {
        // Varargs (usually varcall, but not necessarily -- utilities use ptrcall)

        // If the return type is not Variant, then convert to concrete target type
        let varcall_invocation = &code.varcall_invocation;

        // TODO use Result instead of panic on error
        quote! {
            #safety_doc
            #vis #maybe_unsafe fn #primary_fn_name(
                #receiver_param
                #( #params, )*
                varargs: &[Variant]
            ) #return_decl {
                type CallSig = #call_sig;

                let args = (#( #arg_names, )*);

                unsafe {
                    #varcall_invocation
                }
            }
        }
    } else {
        // Always ptrcall, no varargs

        let ptrcall_invocation = &code.ptrcall_invocation;
        let maybe_return_ty = &sig.return_value().type_;

        // This differentiation is needed because we need to differentiate between Option<Gd<T>>, T and () as return types.
        // Rust traits don't provide specialization and thus would encounter overlapping blanket impls, so we cannot use the type system here.
        let ret_marshal = match maybe_return_ty {
            Some(RustTy::EngineClass { tokens, .. }) => quote! { PtrcallReturnOptionGdT<#tokens> },
            Some(return_ty) => quote! { PtrcallReturnT<#return_ty> },
            None => quote! { PtrcallReturnUnit },
        };

        quote! {
            #safety_doc
            #vis #maybe_unsafe fn #primary_fn_name(
                #receiver_param
                #( #params, )*
            ) #return_decl {
                type RetMarshal = #ret_marshal;
                type CallSig = #call_sig;

                let args = (#( #arg_names, )*);

                unsafe {
                    #ptrcall_invocation
                }
            }
        }
    };

    FnDefinition {
        functions: quote! {
            #primary_function
            #default_fn_code
        },
        builders: default_structs_code,
    }
}

pub fn make_receiver(qualifier: FnQualifier, ffi_arg_in: TokenStream) -> FnReceiver {
    assert_ne!(qualifier, FnQualifier::Global, "expected class");

    let param = match qualifier {
        FnQualifier::Const => quote! { &self, },
        FnQualifier::Mut => quote! { &mut self, },
        FnQualifier::Static => quote! {},
        FnQualifier::Global => quote! {},
    };

    let (ffi_arg, self_prefix);
    if matches!(qualifier, FnQualifier::Static) {
        ffi_arg = quote! { std::ptr::null_mut() };
        self_prefix = quote! { Self:: };
    } else {
        ffi_arg = ffi_arg_in;
        self_prefix = quote! { self. };
    };

    FnReceiver {
        param,
        ffi_arg,
        self_prefix,
    }
}

pub fn make_params_and_args(method_args: &[&FnParam]) -> (Vec<TokenStream>, Vec<TokenStream>) {
    method_args
        .iter()
        .map(|param| {
            let param_name = &param.name;
            let param_ty = &param.type_;

            (quote! { #param_name: #param_ty }, quote! { #param_name })
        })
        .unzip()
}

pub fn make_vis(is_private: bool) -> TokenStream {
    if is_private {
        quote! { pub(crate) }
    } else {
        quote! { pub }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Implementation

fn make_params_exprs(method_args: &[FnParam]) -> [Vec<TokenStream>; 3] {
    let mut params = vec![];
    let mut param_types = vec![];
    let mut arg_names = vec![];

    for param in method_args.iter() {
        let param_name = &param.name;
        let param_ty = &param.type_;

        params.push(quote! { #param_name: #param_ty });
        param_types.push(quote! { #param_ty });
        arg_names.push(quote! { #param_name });
    }

    [params, param_types, arg_names]
}

fn function_uses_pointers(sig: &dyn Function) -> bool {
    let has_pointer_params = sig
        .params()
        .iter()
        .any(|param| matches!(param.type_, RustTy::RawPointer { .. }));

    let has_pointer_return = matches!(sig.return_value().type_, Some(RustTy::RawPointer { .. }));

    // No short-circuiting due to variable decls, but that's fine.
    has_pointer_params || has_pointer_return
}
