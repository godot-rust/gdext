/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use proc_macro2::TokenStream;

use crate::class::{transform_inherent_impl, transform_trait_impl};
use crate::util::{bail, KvParser};
use crate::ParseResult;

use quote::quote;

fn parse_inherent_impl_attr(meta: TokenStream) -> super::InherentImplAttr {
    // Hack because venial doesn't support direct meta parsing yet.
    let input = quote! {
        #[godot_api(#meta)]
        fn () {}
    };

    // todo: properly handle errors instead of 'unwrap'
    
    let item = venial::parse_item(input).unwrap();
    let mut attr = KvParser::parse_required(&item.attributes(), "godot_api", &meta).unwrap();
    let secondary = attr.handle_alone("secondary").unwrap();
    attr.finish().unwrap();

    super::InherentImplAttr {
        secondary: secondary,
    }
}

pub fn attribute_godot_api(
    meta: TokenStream,
    input_decl: venial::Item,
) -> ParseResult<TokenStream> {
    let decl = match input_decl {
        venial::Item::Impl(decl) => decl,
        _ => bail!(
            input_decl,
            "#[godot_api] can only be applied on impl blocks",
        )?,
    };

    if decl.impl_generic_params.is_some() {
        bail!(
            &decl,
            "#[godot_api] currently does not support generic parameters",
        )?;
    }

    if decl.self_ty.as_path().is_none() {
        return bail!(decl, "invalid Self type for #[godot_api] impl");
    };

    if decl.trait_ty.is_some() {
        if meta.to_string() != "" {
            return bail!(
                meta,
                "#[godot_api] on a trait implementation currently does not support any parameters"
            );
        }
        transform_trait_impl(decl)
    } else {
        transform_inherent_impl(parse_inherent_impl_attr(meta), decl)
    }
}
