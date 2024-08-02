/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::generator::functions_common;
use crate::generator::functions_common::FnCode;
use crate::models::domain::{FnParam, FnQualifier, Function, RustTy, TyName};
use crate::util::{ident, safe_ident};
use crate::{conv, special_cases};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};

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
    let vis = functions_common::make_vis(sig.is_private());

    let (builder_doc, surround_class_prefix) = make_extender_doc(sig, &extended_fn_name);

    let ExtenderReceiver {
        object_fn_param,
        object_param,
        object_arg,
    } = make_extender_receiver(sig);

    let Extender {
        builder_ty,
        builder_lifetime,
        builder_anon_lifetime,
        builder_methods,
        builder_fields,
        builder_args,
        builder_inits,
    } = make_extender(sig, object_fn_param, default_fn_params);

    let receiver_param = &code.receiver.param;
    let receiver_self = &code.receiver.self_prefix;

    let [required_params_impl_asarg, _, required_args_asarg] =
        functions_common::make_params_exprs(required_fn_params.iter().cloned(), false, true, true);

    let [required_params_plain, _, required_args_internal] =
        functions_common::make_params_exprs(required_fn_params.into_iter(), false, false, false);

    let return_decl = &sig.return_value().decl;

    // Technically, the builder would not need a lifetime -- it could just maintain an `object_ptr` copy.
    // However, this increases the risk that it is used out of place (not immediately for a default-param call).
    // Ideally we would require &mut, but then we would need `mut Gd<T>` objects everywhere.

    // #[allow] exceptions:
    // - wrong_self_convention:     to_*() and from_*() are taken from Godot
    // - redundant_field_names:     'value: value' is a possible initialization pattern
    // - needless-update:           Remainder expression '..self' has nothing left to change
    let builders = quote! {
        #[doc = #builder_doc]
        #[must_use]
        #cfg_attributes
        pub struct #builder_ty #builder_lifetime {
            // #builder_surround_ref
            #( #builder_fields, )*
        }

        #[allow(clippy::wrong_self_convention, clippy::redundant_field_names, clippy::needless_update)]
        impl #builder_lifetime #builder_ty #builder_lifetime {
            fn new(
                #object_param
                #( #required_params_plain, )*
            ) -> Self {
                Self {
                    #( #builder_inits, )*
                }
            }

            #( #builder_methods )*

            #[inline]
            pub fn done(self) #return_decl {
                #surround_class_prefix #full_fn_name(
                    #( #builder_args, )*
                )
            }
        }
    };

    let functions = quote! {
        #[inline]
        #vis fn #simple_fn_name(
            #receiver_param
            #( #required_params_impl_asarg, )*
        ) #return_decl {
            #receiver_self #extended_fn_name(
                #( #required_args_internal, )*
            ).done()
        }

        #[inline]
        #vis fn #extended_fn_name(
            #receiver_param
            #( #required_params_impl_asarg, )*
        ) -> #builder_ty #builder_anon_lifetime {
            #builder_ty::new(
                #object_arg
                #( #required_args_asarg, )*
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

struct ExtenderReceiver {
    object_fn_param: Option<FnParam>,
    object_param: TokenStream,
    object_arg: TokenStream,
}

struct Extender {
    builder_ty: Ident,
    builder_lifetime: TokenStream,
    builder_anon_lifetime: TokenStream,
    builder_methods: Vec<TokenStream>,
    builder_fields: Vec<TokenStream>,
    builder_args: Vec<TokenStream>,
    builder_inits: Vec<TokenStream>,
}

fn make_extender_doc(sig: &dyn Function, extended_fn_name: &Ident) -> (String, TokenStream) {
    // Not in the above match, because this is true for both static/instance methods.
    // Static/instance is determined by first argument (always use fully qualified function call syntax).
    let surround_class_prefix;
    let builder_doc;

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
                object_param: quote! { surround_object: &'a #builder_mut re_export::#class, },
                object_arg: quote! { self, },
            }
        }
        _ => ExtenderReceiver {
            object_fn_param: None,
            object_param: TokenStream::new(),
            object_arg: TokenStream::new(),
        },
    }
}

fn make_extender(
    sig: &dyn Function,
    object_fn_param: Option<FnParam>,
    default_fn_params: Vec<&FnParam>,
) -> Extender {
    // Note: could build a documentation string with default values here, but the Rust tokens are not very readable,
    // and often not helpful, such as Enum::from_ord(13). Maybe one day those could be resolved and curated.

    let (lifetime, anon_lifetime) = if sig.qualifier().is_static_or_global() {
        (TokenStream::new(), TokenStream::new())
    } else {
        (quote! { <'a> }, quote! { <'_> })
    };

    let all_fn_params = object_fn_param.iter().chain(sig.params().iter());
    let len = all_fn_params.size_hint().0;

    let mut result = Extender {
        builder_ty: format_ident!("Ex{}", conv::to_pascal_case(sig.name())),
        builder_lifetime: lifetime,
        builder_anon_lifetime: anon_lifetime,
        builder_methods: Vec::with_capacity(default_fn_params.len()),
        builder_fields: Vec::with_capacity(len),
        builder_args: Vec::with_capacity(len),
        builder_inits: Vec::with_capacity(len),
    };

    for param in all_fn_params {
        let FnParam {
            name,
            type_,
            default_value,
        } = param;

        let (field_type, needs_conversion) = type_.private_field_decl();

        // Initialize with default parameters where available, forward constructor args otherwise
        let init = if let Some(value) = default_value {
            make_field_init(name, value, needs_conversion)
        } else {
            quote! { #name }
        };

        result.builder_fields.push(quote! { #name: #field_type });
        result.builder_args.push(quote! { self.#name });
        result.builder_inits.push(init);
    }

    for param in default_fn_params {
        let FnParam { name, type_, .. } = param;
        let param_type = type_.param_decl();
        let (_, field_needs_conversion) = type_.private_field_decl();

        let field_init = make_field_init(name, &quote! { value }, field_needs_conversion);

        let method = quote! {
            #[inline]
            pub fn #name(self, value: #param_type) -> Self {
                // Currently not testing whether the parameter was already set
                Self {
                    #field_init,
                    ..self
                }
            }
        };

        result.builder_methods.push(method);
    }

    result
}

fn make_field_init(name: &Ident, expr: &TokenStream, needs_conversion: bool) -> TokenStream {
    if needs_conversion {
        quote! { #name: #expr.as_object_arg() }
    } else {
        quote! { #name: #expr }
    }
}
