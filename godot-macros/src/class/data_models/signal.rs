/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// Some duplication with godot-codegen/signals.rs; see comments there.

use crate::util::bail;
use crate::{util, ParseResult};
use proc_macro2::{Delimiter, Ident, TokenStream, TokenTree};
use quote::{format_ident, quote, ToTokens};

/// Holds information known from a signal's definition
pub struct SignalDefinition {
    /// The signal's function signature (simplified, not original declaration).
    pub fn_signature: venial::Function,

    /// The signal's non-gdext attributes (all except #[signal]).
    pub external_attributes: Vec<venial::Attribute>,

    /// Whether there is going to be a type-safe builder for this signal (true by default).
    pub has_builder: bool,
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Limits the visibility of signals to a few "blessed" syntaxes, excluding `pub(in PATH)`.
///
/// This is necessary because the signal collection (containing all signals) must have the widest visibility of any signal, and for
/// that a total order must exist. `in` paths cannot be semantically analyzed by proc-macros.
///
/// Documented in <https://godot-rust.github.io/book/register/signals.html#signal-visibility>.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub enum SignalVisibility {
    Priv,
    PubSuper,
    PubCrate,
    Pub,
}

impl SignalVisibility {
    pub fn try_parse(tokens: Option<&venial::VisMarker>) -> Option<Self> {
        // No tokens: private.
        let Some(tokens) = tokens else {
            return Some(Self::Priv);
        };

        debug_assert_eq!(tokens.tk_token1.to_string(), "pub");

        // Early exit if `pub` without following `(...)` group.
        let group = match &tokens.tk_token2 {
            None => return Some(Self::Pub),
            Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Parenthesis => group,
            _ => return None,
        };

        // `pub(...)` -> extract `...` part.
        let mut tokens_in_paren = group.stream().into_iter();
        let vis = match tokens_in_paren.next() {
            Some(TokenTree::Ident(ident)) if ident == "super" => Self::PubSuper,
            Some(TokenTree::Ident(ident)) if ident == "crate" => Self::PubCrate,
            _ => return None,
        };

        // No follow-up tokens allowed.
        if tokens_in_paren.next().is_some() {
            return None;
        }

        Some(vis)
    }
}

impl ToTokens for SignalVisibility {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Priv => { /* do nothing */ }
            Self::Pub => tokens.extend(quote! { pub }),
            Self::PubSuper => tokens.extend(quote! { pub(super) }),
            Self::PubCrate => tokens.extend(quote! { pub(crate) }),
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Extracted syntax info for a declared signal.
struct SignalDetails<'a> {
    /// `fn my_signal(i: i32, s: GString)` -- simplified from original declaration.
    fn_signature: &'a venial::Function,
    /// `MyClass`
    #[allow(unused)]
    // Current impl doesn't need it, but we already have it, too annoying to add/remove during refactors.
    class_name: &'a Ident,
    /// `i32`, `GString`
    param_types: Vec<venial::TypeExpr>,
    /// `i`, `s`
    param_names: Vec<Ident>,
    /// `"i"`, `"s"`
    param_names_str: Vec<String>,
    /// `(i32, GString)`
    param_tuple: TokenStream,
    /// `MySignal`
    signal_name: &'a Ident,
    /// `"MySignal"`
    signal_name_str: String,
    /// `#[cfg(..)] #[cfg(..)]`
    signal_cfg_attrs: Vec<&'a venial::Attribute>,
    /// `MyClass_MySignal`
    individual_struct_name: Ident,
    /// Visibility, e.g. `pub(crate)`
    vis_marker: Option<venial::VisMarker>,
    // /// Detected visibility as strongly typed enum.
    // vis_classified: SignalVisibility,
}

impl<'a> SignalDetails<'a> {
    pub fn extract(
        fn_signature: &'a venial::Function, // *Not* the original #[signal], just the signature part (no attributes, body, etc).
        class_name: &'a Ident,
        external_attributes: &'a [venial::Attribute],
    ) -> ParseResult<SignalDetails<'a>> {
        let mut param_types = vec![];
        let mut param_names = vec![];
        let mut param_names_str = vec![];

        for (param, _punct) in fn_signature.params.inner.iter() {
            match param {
                venial::FnParam::Typed(param) => {
                    param_types.push(param.ty.clone());
                    param_names.push(param.name.clone());
                    param_names_str.push(param.name.to_string());
                }
                venial::FnParam::Receiver(receiver) => {
                    return bail!(receiver, "#[signal] cannot have receiver (self) parameter");
                }
            };
        }

        // Transport #[cfg] attributes to the FFI glue, to ensure signals which were conditionally
        // removed from compilation don't cause errors.
        let signal_cfg_attrs = util::extract_cfg_attrs(external_attributes)
            .into_iter()
            .collect();

        let param_tuple = quote! { ( #( #param_types, )* ) };
        let signal_name = &fn_signature.name;
        let individual_struct_name = format_ident!("__godot_Signal_{}_{}", class_name, signal_name);

        let vis_marker = &fn_signature.vis_marker;
        let Some(_vis_classified) = SignalVisibility::try_parse(vis_marker.as_ref()) else {
            return bail!(
                vis_marker,
                "invalid visibility `{}` for #[signal]; supported are `pub`, `pub(crate)`, `pub(super)` and private (no visibility marker)",
                vis_marker.to_token_stream().to_string()
            );
        };

        Ok(Self {
            fn_signature,
            class_name,
            param_types,
            param_names,
            param_names_str,
            param_tuple,
            signal_name,
            signal_name_str: fn_signature.name.to_string(),
            signal_cfg_attrs,
            individual_struct_name,
            vis_marker: vis_marker.clone(),
            // vis_classified,
        })
    }
}

/// Returns tuple of:
/// * Code registering signals with Godot engine.
/// * Symbolic APIs for signals (collection struct + individual signal types + `WithSignal`/`Deref` impls).
pub fn make_signal_registrations(
    signals: &[SignalDefinition],
    class_name: &Ident,
    class_name_obj: &TokenStream,
    no_typed_signals: bool,
) -> ParseResult<(Vec<TokenStream>, Option<TokenStream>)> {
    let mut signal_registrations = Vec::new();

    #[cfg(since_api = "4.2")]
    let mut collection_api = SignalCollection::default();
    // #[cfg(since_api = "4.2")]
    // let mut max_visibility = SignalVisibility::Priv;

    for signal in signals {
        let SignalDefinition {
            fn_signature,
            external_attributes,
            has_builder,
        } = signal;

        let details = SignalDetails::extract(fn_signature, class_name, external_attributes)?;

        // Callable custom functions are only supported in 4.2+, upon which custom signals rely.
        #[cfg(since_api = "4.2")]
        if *has_builder {
            collection_api.extend_with(&details);
            // max_visibility = max_visibility.max(details.vis_classified);
        }

        let registration = make_signal_registration(&details, class_name_obj);
        signal_registrations.push(registration);
    }

    // Rewrite the above using #[cfg].
    #[cfg(since_api = "4.2")]
    let signal_symbols =
        (!no_typed_signals).then(|| make_signal_symbols(class_name, collection_api));

    #[cfg(before_api = "4.2")]
    let signal_symbols = None;

    Ok((signal_registrations, signal_symbols))
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

fn make_signal_registration(details: &SignalDetails, class_name_obj: &TokenStream) -> TokenStream {
    let SignalDetails {
        param_types,
        param_names,
        param_names_str,
        signal_name_str,
        signal_cfg_attrs,
        ..
    } = details;

    let param_list = util::make_signature_param_type(param_types);

    let indexes = 0..param_types.len();
    let param_property_infos = quote! {
        [
            // Don't use raw sys pointers directly; it's very easy to have objects going out of scope.
            #(
                <#param_list as ::godot::meta::ParamTuple>
                    ::property_info(#indexes, #param_names_str).unwrap(),
            )*
        ]
    };

    let signal_parameters_count = param_names.len();

    quote! {
        #(#signal_cfg_attrs)*
        unsafe {
            use ::godot::sys;
            let parameters_info: [::godot::meta::PropertyInfo; #signal_parameters_count] = #param_property_infos;

            let mut parameters_info_sys: [sys::GDExtensionPropertyInfo; #signal_parameters_count] =
                std::array::from_fn(|i| parameters_info[i].property_sys());

            let signal_name = ::godot::builtin::StringName::from(#signal_name_str);

            sys::interface_fn!(classdb_register_extension_class_signal)(
                sys::get_library(),
                #class_name_obj.string_sys(),
                signal_name.string_sys(),
                parameters_info_sys.as_ptr(),
                sys::GDExtensionInt::from(#signal_parameters_count as i64),
            );
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// A collection struct accessible via `.signals()` in the generated impl.
///
/// Also defines individual signal types.
#[derive(Default)]
struct SignalCollection {
    /// The individual `my_signal()` accessors, returning concrete signal types.
    provider_methods: Vec<TokenStream>,

    /// The actual signal definitions, including both `struct` and `impl` blocks.
    individual_structs: Vec<TokenStream>,
}

impl SignalCollection {
    fn extend_with(&mut self, details: &SignalDetails) {
        let SignalDetails {
            signal_name,
            signal_name_str,
            signal_cfg_attrs,
            individual_struct_name,
            vis_marker,
            ..
        } = details;

        self.provider_methods.push(quote! {
            // Deliberately not #[doc(hidden)] for IDE completion.
            #(#signal_cfg_attrs)*
            // Note: this could be `pub` always and would still compile (maybe warning with the following message).
            //   associated function `SignalCollection::my_signal` is reachable at visibility `pub(crate)`
            //
            // However, it would still lead to a compile error when declaring the individual signal struct `pub` (or any other
            // visibility that exceeds the class visibility). So, we can as well declare the visibility here.
            #vis_marker fn #signal_name(&mut self) -> #individual_struct_name<'c, C> {
                #individual_struct_name {
                    __typed: ::godot::register::TypedSignal::<'c, C, _>::extract(&mut self.__internal_obj, #signal_name_str)
                }
            }
        });

        self.individual_structs
            .push(make_signal_individual_struct(details))
    }

    fn is_empty(&self) -> bool {
        self.individual_structs.is_empty()
    }
}

fn make_asarg_params(params: &venial::Punctuated<venial::FnParam>) -> TokenStream {
    // Could be specialized by trying to parse types, but won't be 100% accurate due to lack of semantics (AsArg could be a safe fallback). E.g.:
    // if ty.tokens.iter().any(|tk| matches!(tk, TokenTree::Ident(ident) if ident == "Gd")) {
    //     quote! { impl ::godot::meta::AsObjectArg<#some_inner_ty> }
    // }

    let mut tokens = TokenStream::new();

    for (param, _punct) in params.iter() {
        match param {
            venial::FnParam::Typed(param) => {
                let param_name = &param.name;
                let param_type = &param.ty;

                tokens.extend(quote! {
                    #param_name: impl ::godot::meta::AsArg<#param_type>,
                });
            }
            venial::FnParam::Receiver(_) => {
                unreachable!("signals have no receivers; already checked")
            }
        };
    }

    tokens
}

fn make_signal_individual_struct(details: &SignalDetails) -> TokenStream {
    let emit_params = make_asarg_params(&details.fn_signature.params);

    let SignalDetails {
        // class_name,
        param_names,
        param_tuple,
        signal_cfg_attrs,
        individual_struct_name,
        vis_marker,
        ..
    } = details;

    // Define the individual types + trait impls. The idea was originally to use a module to reduce namespace pollution:
    //   let module_name = format_ident!("__godot_signal_{class_name}_{signal_name}");
    //   #(#signal_cfg_attrs)* pub mod #module_name { use super::*; ... }
    //   #(#signal_cfg_attrs)* pub(crate) use #module_name::#individual_struct_name;
    // However, there are some challenges:
    // - Visibility is a pain to handle: rustc doesn't like re-exporting private symbols as pub, and we can't know the visibility of the
    //   surrounding class struct. Users must explicitly declare #[signal]s `pub` if they want wider visibility; this must not exceed the
    //   visibility of the class itself.
    // - Not yet clear if we should have each signal + related types in separate module. If #[signal] is supported in #[godot_api(secondary)]
    //   impl blocks, then we would have to group them by the impl block. Rust doesn't allow partial modules, so they'd need to have individual
    //   names as well, possibly explicitly chosen by the user.
    //
    // For now, #[doc(hidden)] is used in some places to limit namespace pollution at least in IDEs + docs. This also means that the generated
    // code is less observable by the user. If someone comes up with a good idea to handle all this, let us know :)
    quote! {
        #(#signal_cfg_attrs)*
        #[allow(non_camel_case_types)]
        #[doc(hidden)] // Signal struct is hidden, but the method returning it is not (IDE completion).
        #vis_marker struct #individual_struct_name<'c, C: ::godot::obj::WithSignals> {
            #[doc(hidden)]
            __typed: ::godot::register::TypedSignal<'c, C, #param_tuple>,
        }

        // Concrete convenience API is macro-based; many parts are delegated to TypedSignal via Deref/DerefMut.
        #(#signal_cfg_attrs)*
        impl<C: ::godot::obj::WithSignals> #individual_struct_name<'_, C> {
            pub fn emit(&mut self, #emit_params) {
                use ::godot::meta::AsArg;
                #(
                    ::godot::meta::arg_into_owned!(infer #param_names);
                    //let #param_names = #param_names.into_arg();
                )*

                self.__typed.emit_tuple((#( #param_names, )*));
            }
        }

        #(#signal_cfg_attrs)*
        impl<'c, C: ::godot::obj::WithSignals> std::ops::Deref for #individual_struct_name<'c, C> {
            type Target = ::godot::register::TypedSignal<'c, C, #param_tuple>;

            fn deref(&self) -> &Self::Target {
                &self.__typed
            }
        }

        #(#signal_cfg_attrs)*
        impl<C: ::godot::obj::WithSignals> std::ops::DerefMut for #individual_struct_name<'_, C> {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.__typed
            }
        }
    }
}

/// Generates symbolic API for signals:
/// * collection (unspecified-name struct holding methods to access each signal)
///   * can be absent if the class declares no own #[signal]s.
/// * individual signal types
/// * trait impls
fn make_signal_symbols(
    class_name: &Ident,
    collection_api: SignalCollection,
    // max_visibility: SignalVisibility,
) -> TokenStream {
    // Earlier implementation generated a simplified code when no #[signal] was declared: only WithSignals/WithUserSignals impl, but no own
    // collection, instead the associated type pointing to the base class. This has however some problems:
    // * Part of the reason for user-defined collection is to store UserSignalObject instead of Gd, which can store &mut self.
    //   This is necessary for self.signals().some_base_signal().emit(), if such a signal is connected to Self::method_mut;
    //   Gd would cause a borrow error.
    // * Once we add Rust-Rust inheritance, we'd need to differentiate case again, which can be tricky since #[godot_api] has no information
    //   about the base class.
    //
    // As compromise, we always generate a collection struct if either at least 1 #[signal] is declared or the struct has a Base<T> field.
    // We also provide opt-out via #[godot_api(no_typed_signals)].

    let declares_no_signals = collection_api.is_empty();
    let collection_struct_name = format_ident!("__godot_Signals_{}", class_name);
    let collection_struct_methods = &collection_api.provider_methods;
    let with_signals_impl = make_with_signals_impl(class_name, &collection_struct_name);
    let upcast_deref_impl = make_upcast_deref_impl(class_name, &collection_struct_name);
    let individual_structs = collection_api.individual_structs;

    // The collection cannot be `pub` because `Deref::Target` contains the class type, which leads to "leak private type" errors.
    // We thus adopt the visibility of the #[derive(GodotClass)] struct, imported via macro trick.
    //
    // A previous approach (that cannot access the struct visibility) used "max visibility": the user decides which visibility is acceptable f
    // or individual #[signal]s. They can all be at most the class visibility. Since we assume that decision is correct, the signal collection
    // itself can also share the widest visibility of any #[signal]. This approach however still led to problems because there's a 2-way
    // dependency: `impl WithSignals for MyClass` has an associated type `SignalCollection` that mentions the generated collection type. If
    // that collection type has *lower* visibility than the class, we *also* run into "leak private type" errors.

    // Unrelated, we could use the following for encapsulation:
    //     #[cfg(since_api = "4.2")]
    //     mod #signal_mod_name {
    //         pub use super::*;
    //         ... // all the code below
    //     }
    //     #[cfg(since_api = "4.2")]
    //     pub use #signal_mod_name::*;
    //
    // This now makes signal types/methods invisible to the surrounding scope, so we'd need to adjust visibility in some cases:
    // * private      -> `pub(super)`
    // * `pub(super)` -> pub(in super::super)
    //
    // Benefit of encapsulating would be:
    // * No need for `#[doc(hidden)]` on internal symbols like fields.
    // * #[cfg(since_api = "4.2")] would not need to be repeated. This is less of a problem if the #[cfg] is used inside the macro
    //   instead of generated code.
    // * Less scope pollution (even though names are mangled).
    //
    // Downside is slightly higher complexity and introducing signals in secondary blocks becomes harder (although we could use another
    // module name, we'd need a way to create unique names).

    let visibility_macro = util::format_class_visibility_macro(class_name);

    let mut code = quote! {
        #visibility_macro! {
            #[allow(non_camel_case_types)]
            #[doc(hidden)] // Only on struct, not methods, to allow completion in IDEs.
            struct #collection_struct_name<'c, C> {
                // Hiding necessary because it's in the same scope as the user-defined class, so appearing in IDE completion.
                #[doc(hidden)]
                __internal_obj: Option<::godot::private::UserSignalObject<'c, C>>
            }
        }

        impl<'c, C> #collection_struct_name<'c, C>
        where // bounds: see UserSignalObject::into_typed_signal().
            C: ::godot::obj::WithUserSignals +
               ::godot::obj::WithSignals<__SignalObj<'c> = ::godot::private::UserSignalObject<'c, C>>,
        {
            #( #collection_struct_methods )*
        }

        #with_signals_impl
        #upcast_deref_impl
        #( #individual_structs )*
    };

    // base_field_macro! is a macro that expands to all input tokens if the class declares a Base<T> field, and to nothing otherwise.
    // This makes sure that WithSignals is only implemented for classes with a base field, and avoids compile errors about it.
    // Only when no #[signal] is declared -> otherwise the user explicitly requests it, and a compile error is better to guide them.
    if declares_no_signals {
        let base_field_macro = util::format_class_base_field_macro(class_name);

        code = quote! {
            #base_field_macro! { #code }
        };
    }

    code
}

/// Declare `impl WithSignals` and `impl WithUserSignals` with own signal collection.
fn make_with_signals_impl(class_name: &Ident, collection_struct_name: &Ident) -> TokenStream {
    quote! {
        impl ::godot::obj::WithSignals for #class_name {
            type SignalCollection<'c, C: ::godot::obj::WithSignals> = #collection_struct_name<'c, C>;

            #[doc(hidden)]
            type __SignalObj<'c> = ::godot::private::UserSignalObject<'c, Self>;

            #[doc(hidden)]
            fn __signals_from_external(external: & ::godot::obj::Gd<Self>) -> Self::SignalCollection<'_, Self> {
                Self::SignalCollection {
                    __internal_obj: Some(::godot::private::UserSignalObject::External {
                        gd: external.clone().upcast_object()
                    })
                }
            }
        }

        impl ::godot::obj::WithUserSignals for #class_name {
            fn signals(&mut self) -> Self::SignalCollection<'_, Self> {
                Self::SignalCollection {
                    __internal_obj: Some(::godot::private::UserSignalObject::Internal { self_mut: self })
                }
            }
        }
    }
}

fn make_upcast_deref_impl(class_name: &Ident, collection_struct_name: &Ident) -> TokenStream {
    quote! {
        impl<'c, C: ::godot::obj::WithSignals> std::ops::Deref for #collection_struct_name<'c, C> {
            type Target = <
                <
                    #class_name as ::godot::obj::GodotClass
                >::Base as ::godot::obj::WithSignals
            >::SignalCollection<'c, C>;

            fn deref(&self) -> &Self::Target {
                type Derived = #class_name;
                ::godot::private::signal_collection_to_base::<C, Derived>(self)
            }
        }

        impl<'c, C: ::godot::obj::WithSignals> std::ops::DerefMut for #collection_struct_name<'c, C> {
            fn deref_mut(&mut self) -> &mut Self::Target {
                type Derived = #class_name;
                ::godot::private::signal_collection_to_base_mut::<C, Derived>(self)
            }
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_signal_visibility() {
        #[rustfmt::skip]
        let list = [
            (quote! { pub },                Some(SignalVisibility::Pub)),
            (quote! { pub(crate) },         Some(SignalVisibility::PubCrate)),
            (quote! {},                     Some(SignalVisibility::Priv)),
            (quote! { pub(super) },         Some(SignalVisibility::PubSuper)),
            (quote! { pub(self) },          None), // not supported (equivalent to private)
            (quote! { pub(in crate::foo) }, None),
        ];

        let parsed = list
            .iter()
            .map(|(vis, _)| {
                // Dummy function, because venial has no per-item parser in public API.
                let item = venial::parse_item(quote! {
                    #vis fn f() {}
                });

                let Ok(venial::Item::Function(f)) = item else {
                    panic!("expected function")
                };

                SignalVisibility::try_parse(f.vis_marker.as_ref())
            })
            .collect::<Vec<_>>();

        for ((_, expected), actual) in list.iter().zip(parsed.iter()) {
            assert_eq!(expected, actual);
        }
    }

    #[test]
    fn signal_visibility_order() {
        assert!(SignalVisibility::Pub > SignalVisibility::PubCrate);
        assert!(SignalVisibility::PubCrate > SignalVisibility::PubSuper);
        assert!(SignalVisibility::PubSuper > SignalVisibility::Priv);
    }
}
