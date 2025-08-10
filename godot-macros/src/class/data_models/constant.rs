/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use proc_macro2::{Ident, TokenStream};
use quote::quote;

use crate::util::bail;
use crate::{util, ParseResult};

pub struct ConstDefinition {
    pub raw_constant: venial::Constant,
}

pub fn make_constant_registration(
    consts: Vec<ConstDefinition>,
    class_name: &Ident,
    class_name_obj: &TokenStream,
) -> ParseResult<TokenStream> {
    let mut integer_constant_cfg_attrs = Vec::new();
    let mut integer_constant_names = Vec::new();
    let mut integer_constant_values = Vec::new();

    for constant in consts.iter() {
        let constant = &constant.raw_constant;
        if constant.initializer.is_none() {
            return bail!(constant, "exported const should have initializer");
        };

        let name = &constant.name;

        // In contrast to #[func] and #[signal], we don't remove the attributes from constant signatures
        // within process_godot_constants().
        let cfg_attrs = util::extract_cfg_attrs(&constant.attributes)
            .into_iter()
            .collect::<Vec<_>>();

        // Transport #[cfg] attributes to the FFI glue, to ensure constants which were conditionally removed
        // from compilation don't cause errors.
        integer_constant_cfg_attrs.push(cfg_attrs);
        integer_constant_names.push(constant.name.to_string());
        integer_constant_values.push(quote! { #class_name::#name });
    }

    let tokens = if !integer_constant_names.is_empty() {
        quote! {
            use ::godot::register::private::constant::*;
            use ::godot::meta::ClassName;
            use ::godot::builtin::StringName;

            #(
                #(#integer_constant_cfg_attrs)*
                ExportConstant::new(
                    #class_name_obj,
                    ConstantKind::Integer(
                        IntegerConstant::new(
                            #integer_constant_names,
                            #integer_constant_values
                        )
                    )
                ).register();
            )*
        }
    } else {
        TokenStream::new()
    };

    Ok(tokens)
}
