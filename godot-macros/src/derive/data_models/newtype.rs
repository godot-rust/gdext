/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use proc_macro2::{Literal, TokenStream};
use quote::{ToTokens, quote};

use crate::util::bail;
use crate::{KvParser, ParseResult};

/// Field name of a named struct, or numeric index of a tuple struct.
pub struct FieldIdent(TokenStream);

impl ToTokens for FieldIdent {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.0.to_tokens(tokens);
    }
}

pub struct NewtypeField {
    pub ident: FieldIdent,
    pub ty: venial::TypeExpr,
}

impl NewtypeField {
    fn named(field: &venial::NamedField) -> Self {
        let name = &field.name;
        Self {
            ident: FieldIdent(quote! { #name }),
            ty: field.ty.clone(),
        }
    }

    fn tuple(index: usize, field: &venial::TupleField) -> Self {
        Self {
            ident: FieldIdent(Literal::usize_unsuffixed(index).into_token_stream()),
            ty: field.ty.clone(),
        }
    }
}

/// Stores info from the field of a newtype struct for use in deriving `GodotConvert` and other related traits.
///
/// `NewtypeStruct` must have exactly 1 sized field, and can have an arbitrary amount of ZST fields.
pub struct NewtypeStruct {
    pub sized: NewtypeField,     // Single sized field
    pub zsts: Vec<NewtypeField>, // skipped ZSTs
}

impl NewtypeStruct {
    /// Parses a struct into a newtype struct.
    ///
    /// This will fail if the struct doesn't have exactly one non-skipped field.
    pub fn parse_struct(struct_: &venial::Struct) -> ParseResult<NewtypeStruct> {
        let all_fields = &struct_.fields;

        // Tuple and named fields have no common accessor, so unify them here.
        let fields: Vec<(NewtypeField, &[venial::Attribute])> = match all_fields {
            venial::Fields::Unit => {
                return bail!(
                    all_fields,
                    "GodotConvert expects a struct with a single sized field, unit structs are currently not supported"
                );
            }

            venial::Fields::Tuple(fields) => fields
                .fields
                .iter()
                .enumerate()
                .map(|(index, (f, _))| (NewtypeField::tuple(index, f), f.attributes.as_slice()))
                .collect(),

            venial::Fields::Named(fields) => fields
                .fields
                .iter()
                .map(|(f, _)| (NewtypeField::named(f), f.attributes.as_slice()))
                .collect(),
        };

        let mut sized = None;
        let mut zsts = vec![];

        for (field, attributes) in fields {
            match KvParser::parse(attributes, "godot")? {
                Some(mut parser) => {
                    // `skip` is the only key allowed on fields; reject `#[godot]` without it.
                    if !parser.handle_alone("skip")? {
                        return bail!(all_fields, "expected `#[godot(skip)]` on skipped fields");
                    }
                    parser.finish()?;
                    zsts.push(field);
                }
                None if sized.is_none() => sized = Some(field),
                None => {
                    return bail!(
                        all_fields,
                        "GodotConvert expects a struct with a single sized field, found multiple"
                    );
                }
            }
        }

        let Some(sized) = sized else {
            return bail!(
                all_fields,
                "GodotConvert expects a struct with a single sized field, found none"
            );
        };

        Ok(NewtypeStruct { sized, zsts })
    }

    /// Struct initializers `field: Default::default()` for skipped fields, each with a compile-time size check.
    pub fn make_zst_field_inits(&self) -> TokenStream {
        let idents = self.zsts.iter().map(|field| &field.ident);
        let tys = self.zsts.iter().map(|field| &field.ty);

        // const{} block rather than static_assert!, whose const *item* cannot name the enclosing generic parameters #tys.
        quote! {
            #(
                #idents: {
                    const { ::std::assert!(::std::mem::size_of::<#tys>() == 0, "#[godot(skip)] field must be zero-sized") };
                    ::std::default::Default::default()
                },
            )*
        }
    }
}
