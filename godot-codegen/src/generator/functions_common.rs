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
    cfg_attributes: &TokenStream,
) -> FnDefinition {
    let has_default_params = default_parameters::function_uses_default_params(sig);
    let vis = if has_default_params {
        // Public API mapped by separate function.
        // Needs to be crate-public because default-arg builder lives outside the module.
        quote! { pub(crate) }
    } else {
        make_vis(sig.is_private())
    };

    // Functions are marked unsafe as soon as raw pointers are involved, irrespectively of whether they appear in parameter or return type
    // position. In cases of virtual functions called by Godot, a returned pointer must be valid and of the expected type. It might be possible
    // to only use `unsafe` for pointers in parameters (for outbound calls), and in return values (for virtual calls). Or technically more
    // correct, make the entire trait unsafe as soon as one function can return pointers, but that's very unergonomic and non-local.
    // Thus, let's keep things simple and more conservative.
    let (maybe_unsafe, maybe_safety_doc) = if let Some(safety_doc) = safety_doc {
        (quote! { unsafe }, safety_doc)
    } else if function_uses_pointers(sig) {
        (
            quote! { unsafe },
            quote! {
                /// # Safety
                ///
                /// This method has automatically been marked `unsafe` because it accepts raw pointers as parameters.
                /// If Godot does not document any safety requirements, make sure you understand the underlying semantics.
            },
        )
    } else {
        (TokenStream::new(), TokenStream::new())
    };

    let [params, param_types, arg_names] = make_params_exprs(
        sig.params().iter(),
        sig.is_virtual(),
        !has_default_params, // For *_full function, we don't need impl AsObjectArg<T> parameters
        !has_default_params, // or arg.as_object_arg() calls.
    );

    let rust_function_name_str = sig.name();

    let (primary_fn_name, default_fn_code, default_structs_code);
    if has_default_params {
        primary_fn_name = format_ident!("{}_full", safe_ident(rust_function_name_str));

        (default_fn_code, default_structs_code) =
            default_parameters::make_function_definition_with_defaults(
                sig,
                code,
                &primary_fn_name,
                cfg_attributes,
            );
    } else {
        primary_fn_name = safe_ident(rust_function_name_str);
        default_fn_code = TokenStream::new();
        default_structs_code = TokenStream::new();
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
            #maybe_safety_doc
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

        // TODO Utility functions: update as well.
        if code.receiver.param.is_empty() {
            quote! {
                #maybe_safety_doc
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
            let try_return_decl = &sig.return_value().call_result_decl();
            let try_fn_name = format_ident!("try_{}", rust_function_name_str);

            // Note: all varargs functions are non-static, which is why there are some shortcuts in try_*() argument forwarding.
            // This can be made more complex if ever necessary.

            // A function() may call try_function(), its arguments should not have .as_object_arg().
            let [_, _, arg_names_without_asarg] = make_params_exprs(
                sig.params().iter(),
                false,
                !has_default_params, // For *_full function, we don't need impl AsObjectArg<T> parameters
                false,               // or arg.as_object_arg() calls.
            );

            quote! {
                /// # Panics
                /// This is a _varcall_ method, meaning parameters and return values are passed as `Variant`.
                /// It can detect call failures and will panic in such a case.
                #maybe_safety_doc
                #vis #maybe_unsafe fn #primary_fn_name(
                    #receiver_param
                    #( #params, )*
                    varargs: &[Variant]
                ) #return_decl {
                    Self::#try_fn_name(self, #( #arg_names_without_asarg, )* varargs)
                        .unwrap_or_else(|e| panic!("{e}"))
                }

                /// # Return type
                /// This is a _varcall_ method, meaning parameters and return values are passed as `Variant`.
                /// It can detect call failures and will return `Err` in such a case.
                #maybe_safety_doc
                #vis #maybe_unsafe fn #try_fn_name(
                    #receiver_param
                    #( #params, )*
                    varargs: &[Variant]
                ) #try_return_decl {
                    type CallSig = #call_sig;

                    let args = (#( #arg_names, )*);

                    unsafe {
                        #varcall_invocation
                    }
                }
            }
        }
    } else {
        // Always ptrcall, no varargs

        let ptrcall_invocation = &code.ptrcall_invocation;

        quote! {
            #maybe_safety_doc
            #vis #maybe_unsafe fn #primary_fn_name(
                #receiver_param
                #( #params, )*
            ) #return_decl {
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
pub fn make_vis(is_private: bool) -> TokenStream {
    if is_private {
        quote! { pub(crate) }
    } else {
        quote! { pub }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Implementation

pub(crate) fn make_params_exprs<'a>(
    method_args: impl Iterator<Item = &'a FnParam>,
    is_virtual: bool,
    param_is_impl_asarg: bool,
    arg_is_asarg: bool,
) -> [Vec<TokenStream>; 3] {
    let mut params = vec![];
    let mut param_types = vec![]; // or non-generic params
    let mut arg_names = vec![];

    for param in method_args {
        let param_name = &param.name;
        let param_ty = &param.type_;

        // Objects (Gd<T>) use implicit conversions via AsObjectArg. Only use in non-virtual functions.
        match &param.type_ {
            RustTy::EngineClass {
                object_arg,
                impl_as_object_arg,
                ..
            } if !is_virtual => {
                // Parameter declarations in signature: impl AsObjectArg<T>
                if param_is_impl_asarg {
                    params.push(quote! { #param_name: #impl_as_object_arg });
                } else {
                    params.push(quote! { #param_name: #object_arg });
                }

                // Argument names in function body: arg.as_object_arg() vs. arg
                if arg_is_asarg {
                    arg_names.push(quote! { #param_name.as_object_arg() });
                } else {
                    arg_names.push(quote! { #param_name });
                }

                param_types.push(quote! { #object_arg });
            }

            _ => {
                params.push(quote! { #param_name: #param_ty });
                arg_names.push(quote! { #param_name });
                param_types.push(quote! { #param_ty });
            }
        }
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
