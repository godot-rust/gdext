/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
#[cfg(all(feature = "register-docs", since_api = "4.3"))]
mod extract_docs;
#[cfg(all(feature = "register-docs", since_api = "4.3"))]
mod markdown_converter;

use proc_macro2::{Ident, TokenStream};

use crate::class::{ConstDefinition, Field, FuncDefinition, SignalDefinition};

#[cfg(all(feature = "register-docs", since_api = "4.3"))]
mod docs_generators {
    use quote::quote;

    use super::*;

    pub fn make_struct_docs_registration(
        base: String,
        description: &[venial::Attribute],
        fields: &[Field],
        class_name: &Ident,
        prv: &TokenStream,
    ) -> TokenStream {
        let struct_docs = extract_docs::document_struct(base, description, fields);
        quote! {
            ::godot::sys::plugin_add!(#prv::__GODOT_DOCS_REGISTRY; #prv::DocsPlugin::new::<#class_name>(
                #prv::DocsItem::Struct(
                    #struct_docs
                )
            ));
        }
    }

    pub fn make_trait_docs_registration(
        functions: &[FuncDefinition],
        constants: &[ConstDefinition],
        signals: &[SignalDefinition],
        class_name: &Ident,
        prv: &TokenStream,
    ) -> TokenStream {
        let extract_docs::InherentImplXmlDocs {
            method_xml_elems,
            constant_xml_elems,
            signal_xml_elems,
        } = extract_docs::document_inherent_impl(functions, constants, signals);

        quote! {
            ::godot::sys::plugin_add!(#prv::__GODOT_DOCS_REGISTRY; #prv::DocsPlugin::new::<#class_name>(
                #prv::DocsItem::InherentImpl(#prv::InherentImplDocs {
                    methods_xml: #method_xml_elems,
                    signals_xml: #signal_xml_elems,
                    constants_xml: #constant_xml_elems
                })
            ));
        }
    }

    pub fn make_interface_impl_docs_registration(
        impl_members: &[venial::ImplMember],
        class_name: &Ident,
        prv: &TokenStream,
    ) -> TokenStream {
        let virtual_methods = extract_docs::document_interface_trait_impl(impl_members);

        quote! {
            ::godot::sys::plugin_add!(#prv::__GODOT_DOCS_REGISTRY; #prv::DocsPlugin::new::<#class_name>(
                #prv::DocsItem::ITraitImpl(#virtual_methods)
            ));
        }
    }
}

#[cfg(all(feature = "register-docs", since_api = "4.3"))]
pub use docs_generators::*;

#[cfg(not(all(feature = "register-docs", since_api = "4.3")))]
mod placeholders {
    use super::*;

    pub fn make_struct_docs_registration(
        _base: String,
        _description: &[venial::Attribute],
        _fields: &[Field],
        _class_name: &Ident,
        _prv: &TokenStream,
    ) -> TokenStream {
        TokenStream::new()
    }

    pub fn make_trait_docs_registration(
        _functions: &[FuncDefinition],
        _constants: &[ConstDefinition],
        _signals: &[SignalDefinition],
        _class_name: &Ident,
        _prv: &proc_macro2::TokenStream,
    ) -> TokenStream {
        TokenStream::new()
    }

    pub fn make_interface_impl_docs_registration(
        _impl_members: &[venial::ImplMember],
        _class_name: &Ident,
        _prv: &TokenStream,
    ) -> TokenStream {
        TokenStream::new()
    }
}

#[cfg(not(all(feature = "register-docs", since_api = "4.3")))]
pub use placeholders::*;
