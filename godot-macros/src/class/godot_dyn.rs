/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::class::transform_dyn_trait_impl;
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
            "#[godot_dyn] does not support lifetimes or generic parameters",
        )?;
    }

    let Some(trait_path) = decl.trait_ty.as_ref() else {
        return bail!(
            &decl,
            "#[godot_dyn] requires a trait; it cannot be applied to inherent impl blocks",
        );
    };

    let mut associated_types = vec![];
    for impl_member in &decl.body_items {
        let venial::ImplMember::AssocType(associated_type) = impl_member else {
            continue;
        };
        let Some(type_expr) = &associated_type.initializer_ty else {
            continue;
        };
        let type_name = &associated_type.name;
        associated_types.push(quote! { #type_name = #type_expr })
    }

    let assoc_type_constraints = if associated_types.is_empty() {
        TokenStream::new()
    } else {
        quote! { < #(#associated_types),* > }
    };

    let class_path = &decl.self_ty;
    let prv = quote! { ::godot::private };

    // TODO: Remove this println! when the code is stable.
    eprintln!("Adding dyn trait impl for {class_path:?}: {trait_path:?} {assoc_type_constraints}");

    transform_dyn_trait_impl(
        decl.clone(),
        prv.clone(),
        class_path.clone(),
        trait_path.clone(),
        assoc_type_constraints,
    )
}
