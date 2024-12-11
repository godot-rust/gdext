/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::util;
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

pub fn make_signal_registrations(
    signals: &[SignalDefinition],
    class_name: &Ident,
    class_name_obj: &TokenStream,
) -> (Vec<TokenStream>, TokenStream) {
    let mut signal_registrations = Vec::new();
    let mut struct_fields = Vec::new();
    let mut struct_methods = Vec::new();

    for signal in signals.iter() {
        let SignalDefinition {
            signature,
            external_attributes,
            has_builder,
        } = signal;
        let mut param_types: Vec<venial::TypeExpr> = Vec::new();
        let mut param_names: Vec<Ident> = Vec::new();
        let mut param_names_str: Vec<String> = Vec::new();

        // let mut receiver_mut = None;
        for (param, _punct) in signature.params.inner.iter() {
            match param {
                venial::FnParam::Typed(param) => {
                    param_types.push(param.ty.clone());
                    param_names.push(param.name.clone());
                    param_names_str.push(param.name.to_string());
                }
                venial::FnParam::Receiver(_receiver) => {
                    // receiver_mut = Some(&receiver.tk_mut);
                }
            };
        }

        let signature_tuple = util::make_signature_tuple_type(&quote! { () }, &param_types);
        let signal_param_tuple = quote! { ( #( #param_types, )* ) };

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

        // Transport #[cfg] attributes to the FFI glue, to ensure signals which were conditionally
        // removed from compilation don't cause errors.
        let signal_cfg_attrs: Vec<&venial::Attribute> =
            util::extract_cfg_attrs(external_attributes)
                .into_iter()
                .collect();

        let signal_name = &signature.name;
        let signal_name_str = signal_name.to_string();
        let signal_parameters_count = param_names.len();

        if *has_builder {
            struct_fields.push(quote! {
                #(#signal_cfg_attrs)*
                #signal_name: ::godot::builtin::TypedSignal<#signal_param_tuple>
            });

            let emit_method = format_ident!("{}", signal_name);
            let connect_method = format_ident!("{}_connect", signal_name);
            let emit_params = &signature.params;

            struct_methods.push(quote! {
                #(#signal_cfg_attrs)*
                fn #emit_method(&mut self, #emit_params) {
                    use ::godot::meta::ToGodot;
                    // Potential optimization: encode args as signature-tuple and use direct ptrcall.
                    let varargs = [
                        #( #param_names.to_variant(), )*
                    ];
                    self.object_base.emit_signal(#signal_name_str, &varargs);
                }

                #(#signal_cfg_attrs)*
                fn #connect_method(&self, f: impl FnMut #signal_param_tuple) {}
            });
        }

        let signal_registration = quote! {
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
        };

        signal_registrations.push(signal_registration);
    }

    let struct_code = if struct_fields.is_empty() {
        TokenStream::new()
    } else {
        let struct_name = format_ident!("{}Signals", class_name);
        quote! {
            pub struct #struct_name<'a> {
                // To allow external call in the future (given Gd<T>, not self), this could be an enum with either BaseMut or &mut Gd<T>/&mut T.
                object_base: ::godot::obj::BaseMut<'a, #class_name>,
            }

            impl #struct_name<'_> {
                #( #struct_methods )*
            }

            impl ::godot::obj::cap::WithSignals for #class_name {
                type SignalCollection<'a> = #struct_name<'a>;

                fn emit(&mut self) -> Self::SignalCollection<'_> {
                    Self::SignalCollection {
                        object_base: self.base_mut(),
                    }
                }
            }
        }
    };

    (signal_registrations, struct_code)
}
