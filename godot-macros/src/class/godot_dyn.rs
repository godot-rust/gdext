/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::util::bail;
use crate::ParseResult;
use proc_macro2::TokenStream;
use quote::quote;

pub fn attribute_godot_dyn(input_decl: venial::Item) -> ParseResult<TokenStream> {
    let venial::Item::Impl(decl) = input_decl else {
        return bail!(
            input_decl,
            "#[godot_dyn] can only be applied on impl blocks",
        );
    };

    if decl.impl_generic_params.is_some() {
        bail!(
            &decl,
            "#[godot_dyn] currently does not support generic parameters",
        )?;
    }

    let Some(trait_path) = decl.trait_ty.as_ref() else {
        return bail!(
            &decl,
            "#[godot_dyn] requires a trait; it cannot be applied to inherent impl blocks",
        );
    };

    let class_path = &decl.self_ty;

    let new_code = quote! {
        #decl

        impl ::godot::obj::AsDyn<dyn #trait_path> for #class_path {
            fn dyn_upcast(&self) -> &(dyn #trait_path + 'static) {
                self
            }

            fn dyn_upcast_mut(&mut self) -> &mut (dyn #trait_path + 'static) {
                self
            }
        }
    };

    Ok(new_code)
}
