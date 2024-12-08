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
        } = signal;
        let mut param_types: Vec<venial::TypeExpr> = Vec::new();
        let mut param_names: Vec<String> = Vec::new();

        for param in signature.params.inner.iter() {
            match &param.0 {
                venial::FnParam::Typed(param) => {
                    param_types.push(param.ty.clone());
                    param_names.push(param.name.to_string());
                }
                venial::FnParam::Receiver(_) => {}
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
                        ::param_property_info(#indexes, #param_names),
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

        struct_fields.push(quote! {
            #(#signal_cfg_attrs)*
            #signal_name: ::godot::builtin::TypedSignal<#signal_param_tuple>
        });

        let emit_method = format_ident!("{}", signal_name);
        let connect_method = format_ident!("{}_connect", signal_name);
        let emit_params = &signature.params;
        struct_methods.push(quote! {
            #(#signal_cfg_attrs)*
            fn #emit_method(&self, #emit_params) {}

            #(#signal_cfg_attrs)*
            fn #connect_method(&self, f: impl FnMut #signal_param_tuple) {}
        });

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

    let struct_name = format_ident!("{}Signals", class_name);
    let struct_code = quote! {
        pub struct #struct_name {
            #( #struct_fields, )*
        }

        impl #struct_name {
            #( #struct_methods )*
        }
    };

    (signal_registrations, struct_code)
}
