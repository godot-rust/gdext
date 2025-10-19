/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::context::Context;
use crate::generator::functions_common::make_virtual_param_type;
use crate::models::domain::{Class, ClassLike, ExtensionApi, FnDirection, Function};

pub fn make_virtual_definitions_file(api: &ExtensionApi, ctx: &mut Context) -> TokenStream {
    make_virtual_hashes_for_all_classes(&api.classes, ctx)
}

fn make_virtual_hashes_for_all_classes(all_classes: &[Class], ctx: &mut Context) -> TokenStream {
    let modules = all_classes
        .iter()
        .map(|class| make_virtual_hashes_for_class(class, ctx));

    quote! {
        #![allow(non_snake_case, non_camel_case_types, non_upper_case_globals, unused_imports)]

        #( #modules )*
    }
}

fn make_virtual_hashes_for_class(class: &Class, ctx: &mut Context) -> TokenStream {
    let class_name = class.name();

    // Import all base class hashes via `use` statements.
    let use_base_class = if let Some(base_class) = ctx.inheritance_tree().direct_base(class_name) {
        quote! {
            pub use super::#base_class::*;

            // For type references in `Sig_*` signature tuples:
            pub use crate::builtin::*;
            pub use crate::classes::native::*;
            pub use crate::obj::Gd;
            pub use std::ffi::c_void;
        }
    } else {
        TokenStream::new()
    };

    let constants = class.methods.iter().filter_map(|method| {
        let FnDirection::Virtual {
            #[cfg(since_api = "4.4")]
            hash,
        } = method.direction()
        else {
            return None;
        };

        let rust_name = method.name_ident();
        let godot_name_str = method.godot_name();

        // Generate parameter types, wrapping EngineClass in Option<> just like the trait does
        let param_types = method
            .params()
            .iter()
            .map(|param| make_virtual_param_type(&param.type_, &param.name, method));

        let rust_sig_name = format_ident!("Sig_{rust_name}");
        let sig_decl = quote! {
            // Pub to allow "inheritance" from other modules.
            pub type #rust_sig_name = ( #(#param_types,)* );
        };

        #[cfg(since_api = "4.4")]
        let constant = quote! {
            pub const #rust_name: (&str, u32) = (#godot_name_str, #hash);
            #sig_decl
        };
        #[cfg(before_api = "4.4")]
        let constant = quote! {
            pub const #rust_name: &str = #godot_name_str;
            #sig_decl
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
