/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use proc_macro2::TokenStream;
use quote::quote;
use venial::Declaration;

use crate::util::{decl_get_info, DeclInfo};
use crate::ParseResult;

pub fn derive_godot_convert(decl: Declaration) -> ParseResult<TokenStream> {
    let DeclInfo {
        where_,
        generic_params,
        name,
        ..
    } = decl_get_info(&decl);

    let gen = generic_params.as_ref().map(|x| x.as_inline_args());

    Ok(quote! {
        impl #generic_params ::godot::builtin::meta::GodotConvert for #name #gen #where_ {
            type Via = ::godot::builtin::Variant;
        }
    })
}
