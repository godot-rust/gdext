/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::util::bail;
use crate::{util, ParseResult};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};

/// Holds information known from a signal's definition
pub struct SignalDefinition {
    /// The signal's function signature.
    pub signature: venial::Function,

    /// The signal's non-gdext attributes (all except #[signal]).
    pub external_attributes: Vec<venial::Attribute>,

    /// Whether there is going to be a type-safe builder for this signal (true by default).
    pub has_builder: bool,
}

/// Extracted syntax info for a declared signal.
struct SignalDetails<'a> {
    /// `fn my_signal(i: i32, s: GString)`
    original_decl: &'a venial::Function,
    /// `MyClass`
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
}

impl<'a> SignalDetails<'a> {
    pub fn extract(
        original_decl: &'a venial::Function,
        class_name: &'a Ident,
        external_attributes: &'a [venial::Attribute],
    ) -> ParseResult<SignalDetails<'a>> {
        let mut param_types = vec![];
        let mut param_names = vec![];
        let mut param_names_str = vec![];

        for (param, _punct) in original_decl.params.inner.iter() {
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
        let signal_name = &original_decl.name;
        let individual_struct_name = format_ident!("{}_{}", class_name, signal_name);

        Ok(Self {
            original_decl,
            class_name,
            param_types,
            param_names,
            param_names_str,
            param_tuple,
            signal_name,
            signal_name_str: original_decl.name.to_string(),
            signal_cfg_attrs,
            individual_struct_name,
        })
    }
}

pub fn make_signal_registrations(
    signals: &[SignalDefinition],
    class_name: &Ident,
    class_name_obj: &TokenStream,
) -> ParseResult<(Vec<TokenStream>, Option<TokenStream>)> {
    let mut signal_registrations = Vec::new();
    let mut collection_api = SignalCollection::default();

    for signal in signals {
        let SignalDefinition {
            signature,
            external_attributes,
            has_builder,
        } = signal;

        let details = SignalDetails::extract(signature, class_name, external_attributes)?;

        // Callable custom functions are only supported in 4.2+, upon which custom signals rely.
        #[cfg(since_api = "4.2")]
        if *has_builder {
            collection_api.extend_with(&details);
        }

        let registration = make_signal_registration(&details, class_name_obj);
        signal_registrations.push(registration);
    }

    let struct_code = make_signal_collection(class_name, collection_api);

    Ok((signal_registrations, struct_code))
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

    let signature_tuple = util::make_signature_tuple_type(&quote! { () }, param_types);

    let indexes = 0..param_types.len();
    let param_property_infos = quote! {
        [
            // Don't use raw sys pointers directly; it's very easy to have objects going out of scope.
            #(
                <#signature_tuple as godot::meta::VarcallSignatureTuple>
                    ::param_property_info(#indexes, #param_names_str),
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
    collection_methods: Vec<TokenStream>,

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
            ..
        } = details;

        self.collection_methods.push(quote! {
            #(#signal_cfg_attrs)*
            fn #signal_name(self) -> #individual_struct_name<'a> {
                #individual_struct_name {
                    typed: ::godot::register::TypedSignal::new(self.object, #signal_name_str)
                }
            }
        });

        self.individual_structs
            .push(make_signal_individual_struct(details))
    }

    pub fn is_empty(&self) -> bool {
        self.individual_structs.is_empty()
    }
}

fn make_signal_individual_struct(details: &SignalDetails) -> TokenStream {
    let emit_params = &details.original_decl.params;

    let SignalDetails {
        class_name,
        param_names,
        param_tuple,
        // signal_name,
        signal_cfg_attrs,
        individual_struct_name,
        ..
    } = details;

    // let module_name = format_ident!("__godot_signal_{class_name}_{signal_name}");

    // Module + re-export.
    // Could also keep contained in module to reduce namespace pollution, but might make docs a bit more nested.
    quote! {
        // #(#signal_cfg_attrs)*
        // mod #module_name {

        // TODO make pub without running into "private type `MySignal` in public interface" errors.
        #(#signal_cfg_attrs)*
        #[allow(non_camel_case_types)]
        struct #individual_struct_name<'a> {
            #[doc(hidden)]
            typed: ::godot::register::TypedSignal<'a, #class_name, #param_tuple>,
        }

        // Concrete convenience API is macro-based; many parts are delegated to TypedSignal via Deref/DerefMut.
        #(#signal_cfg_attrs)*
        impl #individual_struct_name<'_> {
            pub fn emit(&mut self, #emit_params) {
                self.typed.emit((#( #param_names, )*));
            }
        }

        #(#signal_cfg_attrs)*
        impl<'a> std::ops::Deref for #individual_struct_name<'a> {
            type Target = ::godot::register::TypedSignal<'a, #class_name, #param_tuple>;

            fn deref(&self) -> &Self::Target {
                &self.typed
            }
        }

        #(#signal_cfg_attrs)*
        impl std::ops::DerefMut for #individual_struct_name<'_> {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.typed
            }
        }

        // #(#signal_cfg_attrs)*
        // pub(crate) use #module_name::#individual_struct_name;
    }
}

// See also make_func_collection().
fn make_signal_collection(class_name: &Ident, collection: SignalCollection) -> Option<TokenStream> {
    if collection.is_empty() {
        return None;
    }

    let collection_struct_name = format_ident!("{}Signals", class_name);
    let collection_struct_methods = &collection.collection_methods;
    let individual_structs = collection.individual_structs;

    let code = quote! {
        #[allow(non_camel_case_types)]
        pub struct #collection_struct_name<'a> {
            // To allow external call in the future (given Gd<T>, not self), this could be an enum with either BaseMut or &mut Gd<T>/&mut T.
            object: ::godot::register::ObjectRef<'a, #class_name>
        }

        impl<'a> #collection_struct_name<'a> {
            #( #collection_struct_methods )*
        }

        impl ::godot::obj::cap::WithSignals for #class_name {
            type SignalCollection<'a> = #collection_struct_name<'a>;

            fn signals(&mut self) -> Self::SignalCollection<'_> {
                Self::SignalCollection {
                    object: ::godot::register::ObjectRef::Internal { obj_mut: self }
                }
            }

            #[doc(hidden)]
            fn __signals_from_external(external: &Gd<Self>) -> Self::SignalCollection<'_> {
                Self::SignalCollection {
                    object: ::godot::register::ObjectRef::External { gd: external.clone() }
                }
            }
        }

        #( #individual_structs )*
    };
    Some(code)
}
