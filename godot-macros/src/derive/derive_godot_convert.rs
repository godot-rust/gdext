/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use proc_macro2::TokenStream;
use quote::quote;
use venial::Declaration;

use crate::ParseResult;

use crate::derive::data_model::GodotConvert;

pub fn derive_godot_convert(declaration: Declaration) -> ParseResult<TokenStream> {
    let GodotConvert { name, data } = GodotConvert::parse_declaration(declaration)?;

    let via_type = data.via_type();

    Ok(quote! {
        impl ::godot::builtin::meta::GodotConvert for #name  {
            type Via = #via_type;
        }
    })
}
