/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::models::domain::{Class, ClassLike, ExtensionApi, FnDirection, Function};
use proc_macro2::TokenStream;
use quote::quote;

pub fn make_virtual_hashes_file(api: &ExtensionApi) -> TokenStream {
    make_virtual_hashes_for_all_classes(&api.classes)
}

fn make_virtual_hashes_for_all_classes(all_classes: &[Class]) -> TokenStream {
    let modules = all_classes
        .iter()
        .map(|class| make_virtual_hashes_for_class(class));

    quote! {
        #![allow(non_snake_case, non_upper_case_globals)]

        #( #modules )*
    }
}

fn make_virtual_hashes_for_class(class: &Class) -> TokenStream {
    let class_rust_name = &class.name().rust_ty;

    let constants: Vec<TokenStream> = class
        .methods
        .iter()
        .filter_map(|method| {
            let FnDirection::Virtual { hash } = method.direction() else {
                return None;
            };

            let method_name = method.name_ident();
            let constant = quote! {
                pub const #method_name: u32 = #hash;
            };

            Some(constant)
        })
        .collect();

    // Don't generate mod SomeClass {} without contents.
    if constants.is_empty() {
        return TokenStream::new();
    }

    quote! {
        pub mod #class_rust_name {
            #( #constants )*
        }
    }
}
