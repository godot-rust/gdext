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

use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};

use crate::context::Context;
use crate::conv;
use crate::models::domain::{Class, ClassLike, ClassSignal, FnParam, ModName, RustTy, TyName};
use crate::util::{ident, safe_ident};

pub struct SignalCodegen {
    pub signal_code: TokenStream,
    pub has_own_signals: bool,
}

pub fn make_class_signals(
    class: &Class,
    signals: &[ClassSignal],
    ctx: &mut Context,
) -> SignalCodegen {
    let all_params: Vec<SignalParams> = signals
        .iter()
        .map(|s| SignalParams::new(&s.parameters))
        .collect();

    let class_name = class.name();

    // If no signals are defined in current class, walk up until we find some.
    let (own_collection_struct, nearest_collection_name, nearest_class, has_own_signals);
    if signals.is_empty() {
        // Use the nearest base class that *has* signals, and store its collection name.
        let nearest = ctx.find_nearest_base_with_signals(class_name);

        // Doesn't define own collection struct if no signals are present (note that WithSignals is still implemented).
        own_collection_struct = TokenStream::new();
        nearest_collection_name = make_collection_name(&nearest);
        nearest_class = Some(nearest);
        has_own_signals = false;
    } else {
        let (code, name) = make_signal_collection(class, signals, &all_params);

        own_collection_struct = code;
        nearest_collection_name = name;
        nearest_class = None;
        has_own_signals = true;
    };

    let signal_types = signals
        .iter()
        .zip(all_params.iter())
        .map(|(signal, params)| make_signal_individual_struct(signal, params));

    let with_signals_impl =
        make_with_signals_impl(class_name, &nearest_collection_name, nearest_class.as_ref());

    let deref_impl =
        has_own_signals.then(|| make_upcast_deref_impl(class_name, &nearest_collection_name));

    let code = quote! {
        pub use signals::*;

        mod signals {
            use crate::obj::{Gd, GodotClass};
            use super::re_export::#class_name;
            use crate::registry::signal::TypedSignal;
            use super::*;

            // These may be empty if the class doesn't define any signals itself.
            #own_collection_struct
            #( #signal_types )*

            // These are always present.
            #with_signals_impl
            #deref_impl
        }
    };

    SignalCodegen {
        signal_code: code,
        has_own_signals,
    }
}

/// Creates `impl WithSignals`.
///
/// Present for every single class, as every class has at least inherited signals (since `Object` has some).
fn make_with_signals_impl(
    class_name: &TyName,
    collection_struct_name: &Ident,
    nearest_class: Option<&TyName>, // None if own class has signals.
) -> TokenStream {
    let base_use_statement = quote! { use crate::obj::WithSignals; };
    let use_statement = if let Some(nearest_class) = nearest_class {
        let module_name = ModName::from_godot(&nearest_class.godot_ty);
        quote! {
            #base_use_statement
            use crate::classes::#module_name::#collection_struct_name;
        }
    } else {
        base_use_statement
    };

    quote! {
        #use_statement
        impl WithSignals for #class_name {
            type SignalCollection<'c, C: WithSignals> = #collection_struct_name<'c, C>;
            type __SignalObj<'c> = Gd<Self>;
            // type __SignalObj<'c, C: WithSignals> = Gd<Self>;

            // During construction, C = Self.
            #[doc(hidden)]
            fn __signals_from_external(gd_ref: & Gd<Self>) -> Self::SignalCollection<'_, Self> {
                Self::SignalCollection {
                    __internal_obj: Some(gd_ref.clone()),
                }
            }
        }
    }
}

// Used outside, to document class with links to this type.
pub fn make_collection_name(class_name: &TyName) -> Ident {
    format_ident!("SignalsOf{}", class_name.rust_ty)
}

fn make_individual_struct_name(signal_name: &str) -> Ident {
    let signal_pascal_name = conv::to_pascal_case(signal_name);
    format_ident!("Sig{}", signal_pascal_name)
}

fn make_signal_collection(
    class: &Class,
    signals: &[ClassSignal],
    params: &[SignalParams],
) -> (TokenStream, Ident) {
    debug_assert!(!signals.is_empty()); // checked outside

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
            pub fn #signal_name(&mut self) -> #individual_struct_name<'c, C> {
                #individual_struct_name {
                    typed: TypedSignal::extract(&mut self.__internal_obj, #signal_name_str)
                }
            }
        }
    });

    let collection_docs = format!(
        "A collection of signals for the [`{c}`][crate::classes::{c}] class.",
        c = class_name.rust_ty
    );

    let code = quote! {
        #[doc = #collection_docs]
        // C is needed for signals of derived classes that are upcast via Deref; C in that class is the derived class.
        pub struct #collection_struct_name<'c, C: WithSignals /* = #class_name */> {
            #[doc(hidden)]
            pub(crate) __internal_obj: Option<C::__SignalObj<'c>>,
        }

        impl<'c, C: WithSignals> #collection_struct_name<'c, C> {
            #( #provider_methods )*
        }
    };

    (code, collection_struct_name)
}

fn make_upcast_deref_impl(class_name: &TyName, collection_struct_name: &Ident) -> TokenStream {
    // Root of hierarchy, no "upcast" derefs.
    if class_name.rust_ty == "Object" {
        return TokenStream::new();
    }

    quote! {
         impl<'c, C: WithSignals> std::ops::Deref for #collection_struct_name<'c, C> {
            // The whole upcast mechanism is based on C remaining the same even through upcast.
            type Target = <
                <
                    #class_name as crate::obj::GodotClass
                >::Base as WithSignals
            >::SignalCollection<'c, C>;

            fn deref(&self) -> &Self::Target {
                type Derived = #class_name;
                crate::private::signal_collection_to_base::<C, Derived>(self)
            }
        }

        impl<'c, C: WithSignals> std::ops::DerefMut for #collection_struct_name<'c, C> {
            fn deref_mut(&mut self) -> &mut Self::Target {
                type Derived = #class_name;
                crate::private::signal_collection_to_base_mut::<C, Derived>(self)
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

    // let class_name = &signal.surrounding_class;
    // let class_ty = quote! { #class_name };
    let param_tuple = quote! { ( #type_list ) };
    let typed_name = format_ident!("Typed{}", individual_struct_name);

    // Embedded in `mod signals`.
    quote! {
        // Reduce tokens to parse by reusing this type definitions.
        type #typed_name<'c, C> = TypedSignal<'c, C, #param_tuple>;

        pub struct #individual_struct_name<'c, C: WithSignals /* = #class_ty */> {
           typed: #typed_name<'c, C>,
        }

        impl<'c, C: WithSignals> #individual_struct_name<'c, C> {
            pub fn emit(&mut self, #param_list) {
                self.typed.emit_tuple( (#name_list) );
            }
        }

        impl<'c, C: WithSignals> std::ops::Deref for #individual_struct_name<'c, C> {
            type Target = #typed_name<'c, C>;

            fn deref(&self) -> &Self::Target {
                &self.typed
            }
        }

        impl<C: WithSignals> std::ops::DerefMut for #individual_struct_name<'_, C> {
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

            write!(formatted_types, "{param_name}: {formatted_ty}").unwrap();
        }

        Self {
            param_list,
            type_list,
            name_list,
            formatted_types,
        }
    }
}
