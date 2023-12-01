/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use venial::{Declaration, StructFields};

use crate::util::{bail, decl_get_info, ident, DeclInfo};
use crate::ParseResult;

pub fn derive_property(decl: Declaration) -> ParseResult<TokenStream2> {
    let DeclInfo {
        name, name_string, ..
    } = decl_get_info(&decl);

    let body_get;
    let body_set;
    let intermediate;

    let enum_ = match decl {
        Declaration::Enum(e) => e,
        Declaration::Struct(s) => {
            return bail!(s.tk_struct, "Property can only be derived on enums for now")
        }
        Declaration::Union(u) => {
            return bail!(u.tk_union, "Property can only be derived on enums for now")
        }
        _ => unreachable!(),
    };

    if enum_.variants.is_empty() {
        return bail!(
            enum_.name,
            "In order to derive Property, enums must have at least one variant"
        );
    } else {
        let mut matches_get = quote! {};
        let mut matches_set = quote! {};
        intermediate = if let Some(attr) = enum_
            .attributes
            .iter()
            .find(|attr| attr.get_single_path_segment() == Some(&ident("repr")))
        {
            attr.value.to_token_stream()
        } else {
            return bail!(
                name,
                "Property can only be derived on enums with an explicit `#[repr(i*/u*)]` type"
            );
        };

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

            let match_content_get;
            let match_content_set;
            match &enum_v.contents {
                StructFields::Unit => {
                    match_content_get = quote! {
                        Self::#v_name => #v_disc,
                    };
                    match_content_set = quote! {
                        #v_disc => Self::#v_name,
                    };
                }
                _ => {
                    return bail!(
                        v_name,
                        "Property can only be derived on enums with only unit variants for now"
                    )
                }
            };
            matches_get = quote! {
                #matches_get
                #match_content_get
            };
            matches_set = quote! {
                #matches_set
                #match_content_set
            };
        }
        body_get = quote! {
            match &self {
                #matches_get
            }
        };
        body_set = quote! {
            *self = match value {
                #matches_set
                _ => panic!("Incorrect conversion from {} to {}", stringify!(#intermediate), #name_string),
            }
        };
    }

    let out = quote! {
        #[allow(unused_parens)]
        impl godot::bind::property::Property for #name {
            type Intermediate = #intermediate;

            fn get_property(&self) -> #intermediate {
                #body_get
            }

            fn set_property(&mut self, value: #intermediate) {
                #body_set
            }
        }
    };
    Ok(out)
}
