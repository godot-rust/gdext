/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use venial::{Declaration, StructFields};

use crate::util::{bail, decl_get_info, DeclInfo};
use crate::ParseResult;

pub fn derive_export(decl: Declaration) -> ParseResult<TokenStream2> {
    let DeclInfo { name, .. } = decl_get_info(&decl);

    let enum_ = match decl {
        Declaration::Enum(e) => e,
        Declaration::Struct(s) => {
            return bail!(s.tk_struct, "Export can only be derived on enums for now")
        }
        Declaration::Union(u) => {
            return bail!(u.tk_union, "Export can only be derived on enums for now")
        }
        _ => unreachable!(),
    };

    let hint_string = if enum_.variants.is_empty() {
        return bail!(
            enum_.name,
            "In order to derive Export, enums must have at least one variant"
        );
    } else {
        let mut hint_string_segments = Vec::new();
        for (enum_v, _) in enum_.variants.inner.iter() {
            let v_name = enum_v.name.clone();
            let v_disc = if let Some(c) = enum_v.value.clone() {
                c.value
            } else {
                return bail!(
                    v_name,
                    "Property can only be derived on enums with explicit discriminants in all their variants"
                );
            };
            let v_disc_trimmed = v_disc
                .to_string()
                .trim_matches(['(', ')'].as_slice())
                .to_string();

            hint_string_segments.push(format!("{v_name}:{v_disc_trimmed}"));

            match &enum_v.contents {
                StructFields::Unit => {}
                _ => {
                    return bail!(
                        v_name,
                        "Property can only be derived on enums with only unit variants for now"
                    )
                }
            };
        }
        hint_string_segments.join(",")
    };

    let out = quote! {
        #[allow(unused_parens)]
        impl godot::bind::property::Export for #name {
            fn default_export_info() -> godot::bind::property::PropertyHintInfo {
                godot::bind::property::PropertyHintInfo {
                    hint: godot::engine::global::PropertyHint::PROPERTY_HINT_ENUM,
                    hint_string: godot::prelude::GString::from(#hint_string),
                }
            }
        }
    };
    Ok(out)
}
