/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use proc_macro2::TokenStream;
use quote::quote;

use crate::models::domain::{ClassConstant, ClassConstantValue};
use crate::util;

pub fn make_constants(constants: &[ClassConstant]) -> TokenStream {
    let definitions = constants.iter().map(make_constant_definition);

    quote! {
        #( #definitions )*
    }
}

fn make_constant_definition(constant: &ClassConstant) -> TokenStream {
    let constant_name = &constant.name;
    let ident = util::ident(constant_name);
    let vis = if constant_name.starts_with("NOTIFICATION_") {
        quote! { pub(crate) }
    } else {
        quote! { pub }
    };

    match constant.value {
        ClassConstantValue::I32(value) => quote! { #vis const #ident: i32 = #value; },
        ClassConstantValue::I64(value) => quote! { #vis const #ident: i64 = #value; },
    }
}
