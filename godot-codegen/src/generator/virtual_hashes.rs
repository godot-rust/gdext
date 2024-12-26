/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::context::Context;
use crate::models::domain::{Class, ClassLike, ExtensionApi, FnDirection, Function};
use proc_macro2::TokenStream;
use quote::quote;

pub fn make_virtual_hashes_file(api: &ExtensionApi, ctx: &mut Context) -> TokenStream {
    make_virtual_hashes_for_all_classes(&api.classes, ctx)
}

fn make_virtual_hashes_for_all_classes(all_classes: &[Class], ctx: &mut Context) -> TokenStream {
    let modules = all_classes
        .iter()
        .map(|class| make_virtual_hashes_for_class(class, ctx));

    quote! {
        #![allow(non_snake_case, non_upper_case_globals, unused_imports)]

        #( #modules )*
    }
}

fn make_virtual_hashes_for_class(class: &Class, ctx: &mut Context) -> TokenStream {
    let class_name = class.name();

    // Import all base class hashes via `use` statements.
    let use_base_class = if let Some(base_class) = ctx.inheritance_tree().direct_base(class_name) {
        quote! {
            pub use super::#base_class::*;
        }
    } else {
        TokenStream::new()
    };

    let constants = class.methods.iter().filter_map(|method| {
        let FnDirection::Virtual { hash } = method.direction() else {
            return None;
        };

        let method_name = method.name_ident();
        let constant = quote! {
            pub const #method_name: u32 = #hash;
        };

        Some(constant)
    });

    // Even if there are no virtual methods, we need to generate the module, to enable base class imports via `use`.
    quote! {
        pub mod #class_name {
            #use_base_class
            #( #constants )*
        }
    }
}
