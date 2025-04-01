/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// Code duplication: while there is some overlap with godot-macros/signal.rs for #[signal] handling, this file here is quite a bit simpler,
// as it only deals with predefined signal definitions (no user-defined #[cfg], visibility, etc). On the other hand, there is documentation
// for these signals, and integration is slightly different due to lack of WithBaseField trait. Nonetheless, some parts could potentially
// be extracted into a future crate shared by godot-codegen and godot-macros.

use crate::context::Context;
use crate::conv;
use crate::models::domain::{Class, ClassLike, ClassSignal, FnParam, RustTy, TyName};
use crate::util::{ident, safe_ident};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};

pub fn make_class_signals(
    class: &Class,
    signals: &[ClassSignal],
    _ctx: &mut Context,
) -> Option<TokenStream> {
    if signals.is_empty() {
        return None;
    }

    let all_params: Vec<SignalParams> = signals
        .iter()
        .map(|s| SignalParams::new(&s.parameters))
        .collect();

    let signal_collection_struct = make_signal_collection(class, signals, &all_params);

    let signal_types = signals
        .iter()
        .zip(all_params.iter())
        .map(|(signal, params)| make_signal_individual_struct(signal, params));

    let class_name = class.name();

    Some(quote! {
        #[cfg(since_api = "4.2")]
        pub use signals::*;

        #[cfg(since_api = "4.2")]
        mod signals {
            use crate::obj::Gd;
            use super::re_export::#class_name;
            use crate::registry::signal::TypedSignal;
            use super::*;

            #signal_collection_struct
            #( #signal_types )*
        }
    })
}

// Used outside, to document class with links to this type.
pub fn make_collection_name(class_name: &TyName) -> Ident {
    format_ident!("SignalsIn{}", class_name.rust_ty)
}

fn make_individual_struct_name(signal_name: &str) -> Ident {
    let signal_pascal_name = conv::to_pascal_case(signal_name);
    format_ident!("Sig{}", signal_pascal_name)
}

fn make_signal_collection(
    class: &Class,
    signals: &[ClassSignal],
    params: &[SignalParams],
) -> TokenStream {
    let class_name = class.name();
    let collection_struct_name = make_collection_name(class_name);

    let provider_methods = signals.iter().zip(params).map(|(sig, params)| {
        let signal_name_str = &sig.name;
        let signal_name = ident(&sig.name);
        let individual_struct_name = make_individual_struct_name(&sig.name);
        let provider_docs = format!("Signature: `({})`", params.formatted_types);

        quote! {
            // Important to return lifetime 'c here, not '_.
            #[doc = #provider_docs]
            pub fn #signal_name(&mut self) -> #individual_struct_name<'c> {
                #individual_struct_name {
                    typed: TypedSignal::new(self.__gd.clone(), #signal_name_str)
                }
            }
        }
    });

    let collection_docs = format!(
        "A collection of signals for the [`{c}`][crate::classes::{c}] class.",
        c = class_name.rust_ty
    );

    quote! {
        #[doc = #collection_docs]
        pub struct #collection_struct_name<'c> {
            __gd: &'c mut Gd<#class_name>,
        }

        impl<'c> #collection_struct_name<'c> {
            #( #provider_methods )*
        }

        impl crate::obj::WithSignals for #class_name {
            type SignalCollection<'c> = #collection_struct_name<'c>;
            #[doc(hidden)]
            type __SignalObject<'c> = Gd<#class_name>;

            #[doc(hidden)]
            fn __signals_from_external(external: &mut Gd<Self>) -> Self::SignalCollection<'_> {
                Self::SignalCollection {
                    __gd: external,
                }
            }
        }
    }
}

fn make_signal_individual_struct(signal: &ClassSignal, params: &SignalParams) -> TokenStream {
    let individual_struct_name = make_individual_struct_name(&signal.name);

    let SignalParams {
        param_list,
        type_list,
        name_list,
        ..
    } = params;

    let class_name = &signal.surrounding_class;
    let class_ty = quote! { #class_name };
    let param_tuple = quote! { ( #type_list ) };
    let typed_name = format_ident!("Typed{}", individual_struct_name);

    // Embedded in `mod signals`.
    quote! {
        // Reduce tokens to parse by reusing this type definitions.
        type #typed_name<'c> = TypedSignal<'c, #class_ty, #param_tuple>;

        pub struct #individual_struct_name<'c> {
           typed: #typed_name<'c>,
        }

        impl<'c> #individual_struct_name<'c> {
            pub fn emit(&mut self, #param_list) {
                self.typed.emit_tuple( (#name_list) );
            }
        }

        impl<'c> std::ops::Deref for #individual_struct_name<'c> {
            type Target = #typed_name<'c>;

            fn deref(&self) -> &Self::Target {
                &self.typed
            }
        }

        impl std::ops::DerefMut for #individual_struct_name<'_> {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.typed
            }
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

struct SignalParams {
    /// `name: Type, ...`
    param_list: TokenStream,

    /// `Type, ...` -- for example inside a tuple type.
    type_list: TokenStream,

    /// `name, ...` -- for example inside a tuple value.
    name_list: TokenStream,

    /// `"name: Type, ..."` in nice format.
    formatted_types: String,
}

impl SignalParams {
    fn new(params: &[FnParam]) -> Self {
        use std::fmt::Write;

        let mut param_list = TokenStream::new();
        let mut type_list = TokenStream::new();
        let mut name_list = TokenStream::new();
        let mut formatted_types = String::new();
        let mut first = true;

        for param in params.iter() {
            let param_name = safe_ident(&param.name.to_string());
            let param_ty = &param.type_;

            param_list.extend(quote! { #param_name: #param_ty, });
            type_list.extend(quote! { #param_ty, });
            name_list.extend(quote! { #param_name, });

            let formatted_ty = match param_ty {
                RustTy::EngineClass { inner_class, .. } => format!("Gd<{inner_class}>"),
                other => other.to_string(),
            };

            if first {
                first = false;
            } else {
                write!(formatted_types, ", ").unwrap();
            }

            write!(formatted_types, "{}: {}", param_name, formatted_ty).unwrap();
        }

        Self {
            param_list,
            type_list,
            name_list,
            formatted_types,
        }
    }
}
