/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use functions_common as fns;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};

use crate::generator::functions_common;
use crate::generator::functions_common::{
    make_arg_expr, make_param_or_field_type, FnArgExpr, FnCode, FnKind, FnParamDecl,
};
use crate::models::domain::{FnParam, FnQualifier, Function, RustTy, TyName};
use crate::util::{ident, safe_ident};
use crate::{conv, special_cases};

pub fn make_function_definition_with_defaults(
    sig: &dyn Function,
    code: &FnCode,
    full_fn_name: &Ident,
    cfg_attributes: &TokenStream,
) -> (TokenStream, TokenStream) {
    let (default_fn_params, required_fn_params): (Vec<_>, Vec<_>) = sig
        .params()
        .iter()
        .partition(|arg| arg.default_value.is_some());

    let simple_fn_name = safe_ident(sig.name());
    let extended_fn_name = format_ident!("{}_ex", simple_fn_name);
    let default_parameter_usage = format!("To set the default parameters, use [`Self::{extended_fn_name}`] and its builder methods.  See [the book](https://godot-rust.github.io/book/godot-api/functions.html#default-parameters) for detailed usage instructions.");
    let vis = functions_common::make_vis(sig.is_private());

    let (builder_doc, surround_class_prefix) = make_extender_doc(sig, &extended_fn_name);

    let ExtenderReceiver {
        object_fn_param,
        object_arg,
    } = make_extender_receiver(sig);

    let Extender {
        builder_ty,
        builder_ctor_params,
        class_method_required_params,
        class_method_required_params_lifetimed,
        class_method_required_args,
        builder_default_variable_decls,
        builder_field_decls,
        builder_field_init_exprs,
        builder_field_names,
        builder_methods,
        full_fn_args,
    } = make_extender(
        sig.name(),
        object_fn_param,
        &required_fn_params,
        &default_fn_params,
    );

    let return_decl = &sig.return_value().decl;

    // If either the builder has a lifetime (non-static/global method), or one of its parameters is a reference,
    // then we need to annotate the _ex() function with an explicit lifetime. Also adjust &self -> &'a self.
    let receiver_self = &code.receiver.self_prefix;
    let simple_receiver_param = &code.receiver.param;
    let extended_receiver_param = &code.receiver.param_lifetime_a;

    let builders = quote! {
        #[doc = #builder_doc]
        #[must_use]
        #cfg_attributes
        #vis struct #builder_ty<'a> {
            _phantom: std::marker::PhantomData<&'a ()>,
            #( #builder_field_decls, )*
        }

        // #[allow] exceptions:
        // - wrong_self_convention:     to_*() and from_*() are taken from Godot
        // - redundant_field_names:     'value: value' is a possible initialization pattern
        // - needless-update:           Remainder expression '..self' has nothing left to change
        #[allow(clippy::wrong_self_convention, clippy::redundant_field_names, clippy::needless_update)]
        impl<'a> #builder_ty<'a> {
            fn new(
                //#object_param
                #( #builder_ctor_params, )*
            ) -> Self {
                #( #builder_default_variable_decls )*
                Self {
                    _phantom: std::marker::PhantomData,
                    #( #builder_field_names: #builder_field_init_exprs, )*
                }
            }

            #( #builder_methods )*

            #[inline]
            pub fn done(self) #return_decl {
                let Self { _phantom, #( #builder_field_names, )* } = self;
                #surround_class_prefix #full_fn_name(
                    #( #full_fn_args, )* // includes `surround_object` if present
                )
            }
        }
    };

    let functions = quote! {
        // Simple function:
        // Lifetime is set if any parameter is a reference.
        #[doc = #default_parameter_usage]
        #[inline]
        #vis fn #simple_fn_name (
            #simple_receiver_param
            #( #class_method_required_params, )*
        ) #return_decl {
            #receiver_self #extended_fn_name(
                #( #class_method_required_args, )*
            ).done()
        }

        // _ex() function:
        // Lifetime is set if any parameter is a reference OR if the method is not static/global (and thus can refer to self).
        #[inline]
        #vis fn #extended_fn_name<'a> (
            #extended_receiver_param
            #( #class_method_required_params_lifetimed, )*
        ) -> #builder_ty<'a> {
            #builder_ty::new(
                #object_arg
                #( #class_method_required_args, )*
            )
        }
    };

    (functions, builders)
}

pub fn function_uses_default_params(sig: &dyn Function) -> bool {
    sig.params().iter().any(|arg| arg.default_value.is_some())
        && !special_cases::is_method_excluded_from_default_params(
            sig.surrounding_class(),
            sig.name(),
        )
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Implementation

#[derive(Default)]
struct ExtenderReceiver {
    object_fn_param: Option<FnParam>,
    object_arg: TokenStream,
}

struct Extender {
    builder_ty: Ident,
    builder_ctor_params: Vec<TokenStream>,
    builder_default_variable_decls: Vec<TokenStream>,
    /// Required parameters for the class' `simple()` and `simple_ex()` public methods.
    builder_field_decls: Vec<TokenStream>,
    builder_field_init_exprs: Vec<TokenStream>,
    builder_field_names: Vec<Ident>,
    builder_methods: Vec<TokenStream>,
    full_fn_args: Vec<TokenStream>,
    class_method_required_params: Vec<TokenStream>,
    /// Same as `class_method_required_params`, but with lifetimes for all arguments (needed for `_ex()` method).
    class_method_required_params_lifetimed: Vec<TokenStream>,
    /// Arguments forwarded by `simple()` and `simple_ex()` public methods.
    class_method_required_args: Vec<TokenStream>,
}

fn make_extender_doc(sig: &dyn Function, extended_fn_name: &Ident) -> (String, TokenStream) {
    // Not in the above match, because this is true for both static/instance methods.
    // Static/instance is determined by first argument (always use fully qualified function call syntax).
    let surround_class_prefix;
    let builder_doc;

    #[allow(clippy::uninlined_format_args)]
    match sig.surrounding_class() {
        Some(TyName { rust_ty, .. }) => {
            surround_class_prefix = quote! { re_export::#rust_ty:: };
            builder_doc = format!(
                "Default-param extender for [`{class}::{method}`][super::{class}::{method}].",
                class = rust_ty,
                method = extended_fn_name,
            );
        }
        None => {
            // There are currently no default parameters for utility functions
            // -> this is currently dead code, but _should_ work if Godot ever adds them.
            surround_class_prefix = TokenStream::new();
            builder_doc = format!(
                "Default-param extender for [`{function}`][super::{function}].",
                function = extended_fn_name
            );
        }
    };

    (builder_doc, surround_class_prefix)
}

fn make_extender_receiver(sig: &dyn Function) -> ExtenderReceiver {
    let builder_mut = match sig.qualifier() {
        FnQualifier::Const | FnQualifier::Static => quote! {},
        FnQualifier::Mut => quote! { mut },
        FnQualifier::Global => {
            unreachable!("default parameters not supported for global methods; {sig}")
        }
    };

    // Treat the object parameter like other parameters, as first in list.
    // Only add it if the method is not global or static.
    match sig.surrounding_class() {
        Some(surrounding_class) if !sig.qualifier().is_static_or_global() => {
            let class = &surrounding_class.rust_ty;

            ExtenderReceiver {
                object_fn_param: Some(FnParam {
                    name: ident("surround_object"),
                    type_: RustTy::ExtenderReceiver {
                        tokens: quote! { &'a #builder_mut re_export::#class },
                    },
                    default_value: None,
                }),
                object_arg: quote! { self, },
            }
        }
        _ => ExtenderReceiver::default(),
    }
}

fn make_extender(
    fn_name: &str,
    receiver_param: Option<FnParam>,
    required_params: &[&FnParam],
    default_params: &[&FnParam],
) -> Extender {
    // Note: could build a documentation string with default values here, but the Rust tokens are not very readable,
    // and often not helpful, such as Enum::from_ord(13). Maybe one day those could be resolved and curated.

    let all_fn_params = receiver_param
        .iter()
        .chain(required_params.iter().cloned())
        .chain(default_params.iter().cloned());

    // If builder is a method with a receiver OR any *required* parameter is by-ref, use lifetime.
    // Default parameters cannot be by-ref, since they need to store a default value. Potential optimization later.
    let param_decl = FnParamDecl::FnPublicLifetime;
    let ctor_decl = FnKind::ExBuilderConstructorLifetimed;
    let default_len = default_params.len();

    let public_required =
        fns::make_params_exprs(required_params.iter().cloned(), FnKind::DefaultSimpleOrEx);
    let class_method_required_params = public_required.param_decls;
    let class_method_required_args = public_required.arg_exprs;

    let class_method_required_params_lifetimed = fns::make_params_exprs(
        required_params.iter().cloned(),
        FnKind::DefaultSimpleOrExLifetimed,
    )
    .param_decls;

    let receiver_and_required_params = receiver_param.iter().chain(required_params.iter().cloned());
    let ctor_requireds = fns::make_params_exprs(receiver_and_required_params.clone(), ctor_decl);
    let builder_ctor_params = ctor_requireds.param_decls;
    let builder_field_names = all_fn_params
        .clone()
        .map(|param| param.name.clone())
        .collect();

    // Append default arguments.
    let builder_field_init_exprs = {
        let mut builder_field_init_exprs = ctor_requireds.arg_exprs;
        let ctor_defaults = fns::make_params_exprs(
            default_params.iter().cloned(),
            FnKind::ExBuilderConstructorDefault,
        );
        builder_field_init_exprs.extend(ctor_defaults.arg_exprs);
        builder_field_init_exprs
    };

    let mut builder_default_variable_decls = Vec::with_capacity(default_len);
    let mut builder_methods = Vec::with_capacity(default_len);
    for param in default_params.iter() {
        // Declare variable to initialize a default value in the Ex constructor.
        let default_value = param.default_value.as_ref().expect("default value absent");
        let FnParam { name, type_, .. } = param;

        let variable_decl = quote! {
            let #name = #default_value;
        };

        // Gather parameter information for public builder methods.
        let mut dummy_lifetime_gen = fns::LifetimeGen::new();
        let (param_decl, _param_callsig_ty) =
            make_param_or_field_type(name, type_, param_decl, &mut dummy_lifetime_gen);

        let arg_expr = make_arg_expr(name, type_, FnArgExpr::StoreInField);

        let method = quote! {
            #[inline]
            pub fn #name(self, #param_decl) -> Self {
                // Currently not testing whether the parameter was already set.
                Self {
                    #name: #arg_expr,
                    ..self
                }
            }
        };

        builder_default_variable_decls.push(variable_decl);
        builder_methods.push(method);
    }

    let done_fn = fns::make_params_exprs(all_fn_params, FnKind::ExBuilderDone);
    let builder_field_decls = done_fn.param_decls;
    let full_fn_args = done_fn.arg_exprs;

    Extender {
        builder_ty: format_ident!("Ex{}", conv::to_pascal_case(fn_name)),
        builder_ctor_params,
        builder_default_variable_decls,
        builder_methods,
        builder_field_decls,
        builder_field_names,
        full_fn_args,
        builder_field_init_exprs,
        class_method_required_params,
        class_method_required_params_lifetimed,
        class_method_required_args,
    }
}
