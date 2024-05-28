/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use proc_macro2::TokenStream;
use quote::quote;

use crate::derive::data_models::GodotConvert;
use crate::derive::{make_fromgodot, make_togodot};
use crate::ParseResult;

/// Derives `GodotConvert` for the given declaration.
///
/// This also derives `FromGodot` and `ToGodot`.
pub fn derive_godot_convert(item: venial::Item) -> ParseResult<TokenStream> {
    let convert = GodotConvert::parse_declaration(item)?;

    let name = &convert.ty_name;
    let via_type = convert.convert_type.via_type();

    let to_godot_impl = make_togodot(&convert);
    let from_godot_impl = make_fromgodot(&convert);

    Ok(quote! {
        impl ::godot::meta::GodotConvert for #name  {
            type Via = #via_type;
        }

        #to_godot_impl
        #from_godot_impl
    })
}
