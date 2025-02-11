/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::util;
use proc_macro2::TokenStream;
use quote::quote;

/// Holds information known from a signal's definition
pub struct SignalDefinition {
    /// The signal's function signature.
    pub signature: venial::Function,

    /// The signal's non-gdext attributes (all except #[signal]).
    pub external_attributes: Vec<venial::Attribute>,
}

pub fn make_signal_registrations(
    signals: Vec<SignalDefinition>,
    class_name_obj: &TokenStream,
) -> Vec<TokenStream> {
    let mut signal_registrations = Vec::new();

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

        let param_list = quote! { (#(#param_types,)*) };
        let signature_tuple = util::make_signature_tuple_type(&quote! { () }, &param_types);
        let indexes = 0..param_types.len();
        let param_array_decl = quote! {
            [
                // Don't use raw sys pointers directly; it's very easy to have objects going out of scope.
                #(
                    <#param_list as godot::meta::ParamList>
                        ::property_info(#indexes, #param_names),
                )*
            ]
        };

        // Transport #[cfg] attributes to the FFI glue, to ensure signals which were conditionally
        // removed from compilation don't cause errors.
        let signal_cfg_attrs: Vec<&venial::Attribute> =
            util::extract_cfg_attrs(external_attributes)
                .into_iter()
                .collect();
        let signal_name_str = signature.name.to_string();
        let signal_parameters_count = param_names.len();
        let signal_parameters = param_array_decl;

        let signal_registration = quote! {
            #(#signal_cfg_attrs)*
            unsafe {
                use ::godot::sys;
                let parameters_info: [::godot::meta::PropertyInfo; #signal_parameters_count] = #signal_parameters;

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
    signal_registrations
}
