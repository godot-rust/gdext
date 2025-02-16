/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::generator::default_parameters;
use crate::models::domain::{ArgPassing, FnParam, FnQualifier, Function, RustTy};
use crate::special_cases;
use crate::util::lifetime;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};

pub struct FnReceiver {
    /// `&self`, `&mut self`, (none)
    pub param: TokenStream,

    /// `&'a self`, `&'a mut self`, (none)
    pub param_lifetime_a: TokenStream,

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
            param_lifetime_a: TokenStream::new(),
            ffi_arg: TokenStream::new(),
            self_prefix: TokenStream::new(),
        }
    }
}

pub struct FnCode {
    pub receiver: FnReceiver,
    pub varcall_invocation: TokenStream,
    pub ptrcall_invocation: TokenStream,
    pub is_virtual_required: bool,
    pub is_varcall_fallible: bool,
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
        let builder_structs = definitions.iter().map(|def| &def.builders);

        FnDefinitions {
            functions: quote! { #( #functions )* },
            builders: quote! { #( #builder_structs )* },
        }
    }
}

// Gathers multiple token vectors related to function parameters.
#[derive(Default)]
pub struct FnParamTokens {
    pub param_decls: Vec<TokenStream>,
    pub callsig_param_types: Vec<TokenStream>,
    /// Generic argument list `<'a0, 'a1, ...>` after `type CallSig`, if available.
    pub callsig_lifetime_args: Option<TokenStream>,
    pub arg_exprs: Vec<TokenStream>,
    pub func_general_lifetime: Option<TokenStream>,
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

    let FnParamTokens {
        param_decls: params,
        callsig_param_types: param_types,
        callsig_lifetime_args,
        arg_exprs: arg_names,
        func_general_lifetime: fn_lifetime,
    } = if sig.is_virtual() {
        make_params_exprs_virtual(sig.params().iter(), sig)
    } else {
        // primary_function() if not default-params, or full_function() otherwise.
        let passing = if has_default_params {
            FnKind::DefaultFull
        } else {
            FnKind::Regular
        };

        make_params_exprs(sig.params().iter(), passing)
    };

    let rust_function_name = sig.name_ident();

    let (primary_fn_name, default_fn_code, default_structs_code);
    if has_default_params {
        primary_fn_name = format_ident!("{}_full", rust_function_name);

        (default_fn_code, default_structs_code) =
            default_parameters::make_function_definition_with_defaults(
                sig,
                code,
                &primary_fn_name,
                cfg_attributes,
            );
    } else {
        primary_fn_name = rust_function_name.clone();
        default_fn_code = TokenStream::new();
        default_structs_code = TokenStream::new();
    };

    let call_sig_decl = {
        let return_ty = &sig.return_value().type_tokens();

        // Build <'a0, 'a1, ...> for lifetimes.
        quote! {
            type CallSig #callsig_lifetime_args = ( #return_ty, #(#param_types),* );
        }
    };

    let return_decl = &sig.return_value().decl;
    let fn_body = if code.is_virtual_required {
        quote! { ; }
    } else {
        quote! { { unimplemented!() } }
    };

    let receiver_param = &code.receiver.param;
    let primary_function = if sig.is_virtual() {
        // Virtual functions

        quote! {
            #maybe_safety_doc
            #maybe_unsafe fn #primary_fn_name (
                #receiver_param
                #( #params, )*
            ) #return_decl #fn_body
        }
    } else if sig.is_vararg() {
        // Varargs (usually varcall, but not necessarily -- utilities use ptrcall)

        // If the return type is not Variant, then convert to concrete target type
        let varcall_invocation = &code.varcall_invocation;

        // TODO Utility functions: update as well.
        if !code.is_varcall_fallible {
            quote! {
                #maybe_safety_doc
                #vis #maybe_unsafe fn #primary_fn_name (
                    #receiver_param
                    #( #params, )*
                    varargs: &[Variant]
                ) #return_decl {
                    #call_sig_decl

                    let args = (#( #arg_names, )*);

                    unsafe {
                        #varcall_invocation
                    }
                }
            }
        } else {
            let try_return_decl = &sig.return_value().call_result_decl();
            let try_fn_name = format_ident!("try_{}", rust_function_name);

            // Note: all varargs functions are non-static, which is why there are some shortcuts in try_*() argument forwarding.
            // This can be made more complex if ever necessary.

            // A function() may call try_function(), its arguments should not have .as_object_arg().
            let FnParamTokens {
                arg_exprs: arg_names_without_asarg,
                ..
            } = make_params_exprs(sig.params().iter(), FnKind::DelegateTry);

            quote! {
                /// # Panics
                /// This is a _varcall_ method, meaning parameters and return values are passed as `Variant`.
                /// It can detect call failures and will panic in such a case.
                #maybe_safety_doc
                #vis #maybe_unsafe fn #primary_fn_name (
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
                    #call_sig_decl

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
            #vis #maybe_unsafe fn #primary_fn_name #fn_lifetime (
                #receiver_param
                #( #params, )*
            ) #return_decl {
                #call_sig_decl

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

    let (param, param_lifetime_a) = match qualifier {
        FnQualifier::Const => (quote! { &self, }, quote! { &'a self, }),
        FnQualifier::Mut => (quote! { &mut self, }, quote! { &'a mut self, }),
        FnQualifier::GdSelf => (quote! { this: Gd<Self>, }, quote! { this: Gd<Self>, }),
        FnQualifier::Static => (quote! {}, quote! {}),
        FnQualifier::Global => (quote! {}, quote! {}),
    };

    let (ffi_arg, self_prefix);
    if matches!(qualifier, FnQualifier::Static) {
        ffi_arg = quote! { std::ptr::null_mut() };
        self_prefix = quote! { Self:: };
    } else if matches!(qualifier, FnQualifier::Static) {
        ffi_arg = ffi_arg_in;
        self_prefix = quote! { this. };
    } else {
        ffi_arg = ffi_arg_in;
        self_prefix = quote! { self. };
    };

    FnReceiver {
        param,
        param_lifetime_a,
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

#[derive(Copy, Clone)]
pub(crate) enum FnKind {
    /// Most methods.
    Regular,

    /// For default args, the private `some_func_full()` variant.
    DefaultFull,

    /// `some_func()` and `some_func_ex()` forwarding their arguments to `some_func_full()`.
    DefaultSimpleOrEx,

    /// Same as [`DefaultSimpleOrEx`], but with explicit lifetimes.
    DefaultSimpleOrExLifetimed,

    /// `call()` forwarding to `try_call()`.
    DelegateTry,

    /// Default extender `new()` associated function -- optional receiver and required parameters.
    ExBuilderConstructor,

    /// Same as [`ExBuilderConstructor`], but for a builder with an explicit lifetime.
    ExBuilderConstructorLifetimed,

    /// Default extender `new()` associated function -- only default parameters.
    ExBuilderConstructorDefault,

    /// Default extender `done()` method.
    ExBuilderDone,
}

/// How arguments are referred to inside a function.
#[derive(Copy, Clone)]
pub(crate) enum FnArgExpr {
    /// Pass the value to a Godot engine API, i.e. `v` or `v.as_ref()`.
    PassToFfi,

    /// Pass a value to Godot, but from an extender's field (often a `CowArg`).
    PassToFfiFromEx,

    /// Forward the value to another function, i.e. `v`.
    Forward,

    /// Store in a field, i.e. `v` or `v.into_arg()`.
    StoreInField,

    /// Store in a field without coming from a parameter, e.g. `v` or `CowArg::Owned(v)`.
    StoreInDefaultField,
}

/// How parameters are declared in a function signature.
#[derive(Copy, Clone)]
pub(crate) enum FnParamDecl {
    /// Public-facing, i.e. `T`, `&T`, `impl AsArg<T>` or `impl AsObjectArg<T>`.
    FnPublic,

    /// Public-facing with explicit lifetime, e.g. `&'a T`. Used in `Ex` builder methods.
    FnPublicLifetime,

    /// Parameters in internal methods, used for delegation.
    FnInternal,

    /// Store in a field, i.e. `v`, `CowArg<T>` or `ObjectCow<T>`.
    Field,
}

pub(crate) struct LifetimeGen {
    count: usize,
}

impl LifetimeGen {
    pub fn new() -> Self {
        LifetimeGen { count: 0 }
    }

    fn next(&mut self) -> TokenStream {
        let lft = lifetime(&format!("a{}", self.count));
        self.count += 1;
        lft
    }

    fn all_generic_args(&self) -> Option<TokenStream> {
        // No lifetimes needed: we don't have `< >`.
        if self.count == 0 {
            return None;
        }

        let mut tokens = quote! { < };
        for i in 0..self.count {
            let lft = lifetime(&format!("a{}", i));
            tokens.extend(quote! { #lft, });
        }
        tokens.extend(quote! { > });

        Some(tokens)
    }
}

pub(crate) fn make_param_or_field_type(
    name: &Ident,
    ty: &RustTy,
    decl: FnParamDecl,
    lifetimes: &mut LifetimeGen,
) -> (TokenStream, TokenStream) {
    let mut special_ty = None;

    let param_ty = match ty {
        // Objects: impl AsObjectArg<T>
        RustTy::EngineClass {
            object_arg,
            impl_as_object_arg,
            inner_class,
            ..
        } => {
            special_ty = Some(quote! { #object_arg });

            match decl {
                FnParamDecl::FnPublic => quote! { #impl_as_object_arg },
                FnParamDecl::FnPublicLifetime => quote! { #impl_as_object_arg },
                FnParamDecl::FnInternal => quote! { #object_arg },
                FnParamDecl::Field => quote! { ObjectCow<crate::classes::#inner_class> },
            }
        }

        // Strings: impl AsArg<T>
        RustTy::BuiltinIdent {
            arg_passing: ArgPassing::ImplAsArg,
            ..
        } => {
            let lft = lifetimes.next();
            special_ty = Some(quote! { CowArg<#lft, #ty> });

            match decl {
                FnParamDecl::FnPublic => quote! { impl AsArg<#ty> },
                FnParamDecl::FnPublicLifetime => quote! { impl AsArg<#ty> + 'a },
                FnParamDecl::FnInternal => quote! { CowArg<#ty> },
                FnParamDecl::Field => quote! { CowArg<'a, #ty> },
            }
        }

        // By-ref: Array, Dictionary, Variant, Callable, ...
        RustTy::BuiltinIdent {
            arg_passing: ArgPassing::ByRef,
            ..
        }
        | RustTy::BuiltinArray { .. }
        | RustTy::EngineArray { .. } => {
            let lft = lifetimes.next();
            special_ty = Some(quote! { RefArg<#lft, #ty> });

            match decl {
                FnParamDecl::FnPublic => quote! { & #ty },
                FnParamDecl::FnPublicLifetime => quote! { &'a #ty },
                FnParamDecl::FnInternal => quote! { RefArg<#ty> },
                FnParamDecl::Field => quote! { CowArg<'a, #ty>  },
            }
        }

        // By value.
        _ => {
            quote! { #ty }
        }
    };

    let param_decl = quote! { #name: #param_ty };
    let param_ty = special_ty.unwrap_or(param_ty);

    (param_decl, param_ty)
}

pub(crate) fn make_arg_expr(name: &Ident, ty: &RustTy, expr: FnArgExpr) -> TokenStream {
    match ty {
        // Objects.
        RustTy::EngineClass { .. } => match expr {
            FnArgExpr::PassToFfi => quote! { #name.as_object_arg() },
            FnArgExpr::PassToFfiFromEx => quote! { #name.cow_as_object_arg() },
            FnArgExpr::Forward => quote! { #name },
            FnArgExpr::StoreInField => quote! { #name.consume_arg() },
            FnArgExpr::StoreInDefaultField => quote! { #name.consume_arg() },
        },

        // Strings.
        RustTy::BuiltinIdent {
            arg_passing: ArgPassing::ImplAsArg,
            ..
        } => match expr {
            FnArgExpr::PassToFfi => quote! { #name.into_arg() },
            FnArgExpr::PassToFfiFromEx => quote! { #name }, // both field and parameter types are Cow -> forward.
            FnArgExpr::Forward => quote! { #name },
            FnArgExpr::StoreInField => quote! { #name.into_arg() },
            FnArgExpr::StoreInDefaultField => quote! { CowArg::Owned(#name) },
        },

        // By-ref: Array, Dictionary, Variant, Callable, ...
        RustTy::BuiltinIdent {
            arg_passing: ArgPassing::ByRef,
            ..
        }
        | RustTy::BuiltinArray { .. }
        | RustTy::EngineArray { .. } => match expr {
            FnArgExpr::PassToFfi => quote! { RefArg::new(#name) },
            FnArgExpr::PassToFfiFromEx => quote! { #name.cow_as_arg() },
            FnArgExpr::Forward => quote! { #name },
            FnArgExpr::StoreInField => quote! { CowArg::Borrowed(#name) },
            FnArgExpr::StoreInDefaultField => quote! { CowArg::Owned(#name) },
        },

        // By value.
        _ => {
            quote! { #name }
        }
    }
}

/// For non-virtual functions, returns the parameter declarations, type tokens, and names.
pub(crate) fn make_params_exprs<'a>(
    method_args: impl Iterator<Item = &'a FnParam>,
    fn_kind: FnKind,
) -> FnParamTokens {
    let (param_kind, arg_kind) = match fn_kind {
        // Public-facing methods.
        FnKind::Regular => (FnParamDecl::FnPublic, FnArgExpr::PassToFfi),
        FnKind::DefaultSimpleOrEx => (FnParamDecl::FnPublic, FnArgExpr::Forward),
        FnKind::DefaultSimpleOrExLifetimed => (FnParamDecl::FnPublicLifetime, FnArgExpr::Forward),
        FnKind::DelegateTry => (FnParamDecl::FnPublic, FnArgExpr::Forward),

        // Methods relevant in the context of default parameters. Flow in this order.
        // Note that for builder methods of Ex* structs, there's a direct call in default_parameters.rs to the parameter manipulation methods,
        // bypassing this method. So one case is missing here.
        FnKind::ExBuilderConstructor => (FnParamDecl::FnPublic, FnArgExpr::StoreInField),
        FnKind::ExBuilderConstructorLifetimed => {
            (FnParamDecl::FnPublicLifetime, FnArgExpr::StoreInField)
        }
        FnKind::ExBuilderConstructorDefault => {
            (FnParamDecl::FnPublic, FnArgExpr::StoreInDefaultField)
        }
        FnKind::ExBuilderDone => (FnParamDecl::Field, FnArgExpr::PassToFfiFromEx),
        FnKind::DefaultFull => (FnParamDecl::FnInternal, FnArgExpr::Forward),
    };

    let mut ret = FnParamTokens::default();
    let mut lifetime_gen = LifetimeGen::new();

    for param in method_args {
        let param_name = &param.name;
        let param_rust_ty = &param.type_;

        let (param_decl, param_ty) =
            make_param_or_field_type(param_name, param_rust_ty, param_kind, &mut lifetime_gen);
        let arg_expr = make_arg_expr(param_name, param_rust_ty, arg_kind);

        ret.param_decls.push(param_decl);
        ret.arg_exprs.push(arg_expr);
        ret.callsig_param_types.push(param_ty);
    }

    ret.callsig_lifetime_args = lifetime_gen.all_generic_args();
    ret
}

/// For virtual functions, returns the parameter declarations, type tokens, and names.
pub(crate) fn make_params_exprs_virtual<'a>(
    method_args: impl Iterator<Item = &'a FnParam>,
    function_sig: &dyn Function,
) -> FnParamTokens {
    let mut ret = FnParamTokens::default();

    for param in method_args {
        let param_name = &param.name;
        let param_ty = &param.type_;

        match &param.type_ {
            // Virtual methods accept Option<Gd<T>>, since we don't know whether objects are nullable or required.
            RustTy::EngineClass { .. }
                if !special_cases::is_class_method_param_required(
                    function_sig.surrounding_class().unwrap(),
                    function_sig.name(),
                    param_name,
                ) =>
            {
                ret.param_decls
                    .push(quote! { #param_name: Option<#param_ty> });
                ret.arg_exprs.push(quote! { #param_name });
                ret.callsig_param_types.push(quote! { #param_ty });
            }

            // All other methods and parameter types: standard handling.
            // For now, virtual methods always receive their parameter by value.
            //_ => ret.push_regular(param_name, param_ty, true, false, false),
            _ => {
                ret.param_decls.push(quote! { #param_name: #param_ty });
                ret.arg_exprs.push(quote! { #param_name });
                ret.callsig_param_types.push(quote! { #param_ty });
            }
        }
    }

    ret
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
