/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use proc_macro2::{Ident, Literal, TokenStream};
use quote::{ToTokens, quote};

use crate::util::bail;
use crate::{KvParser, ParseResult};

pub struct FieldIdent(TokenStream);

impl FieldIdent {
    fn named(id: Ident) -> Self {
        Self(quote! { #id })
    }
    fn tuple(i: usize) -> Self {
        Self(Literal::usize_unsuffixed(i).into_token_stream())
    }
}

impl ToTokens for FieldIdent {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.0.to_tokens(tokens);
    }
}

pub struct NewtypeField {
    pub ident: FieldIdent,
    pub ty: venial::TypeExpr,
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
    /// This will fail if the struct doesn't have exactly one field.
    pub fn parse_struct(struct_: &venial::Struct) -> ParseResult<NewtypeStruct> {
        match &struct_.fields {
            venial::Fields::Unit => bail!(
                &struct_.fields,
                "GodotConvert expects a struct with a single sized field, unit structs are currently not supported"
            ),
            venial::Fields::Tuple(fields) => {
                let (sized, zsts) = Self::partition_fields(
                    fields
                        .fields
                        .iter()
                        .map(|(field, _)| field.attributes.as_slice()),
                    fields,
                )?;

                let mk = |i: usize| NewtypeField {
                    ident: FieldIdent::tuple(i),
                    ty: fields.fields[i].0.ty.clone(),
                };

                Ok(NewtypeStruct {
                    sized: mk(sized),
                    zsts: zsts.into_iter().map(mk).collect(),
                })
            }
            venial::Fields::Named(fields) => {
                let (sized, zsts) = Self::partition_fields(
                    fields
                        .fields
                        .iter()
                        .map(|(field, _)| field.attributes.as_slice()),
                    fields,
                )?;

                let mk = |i: usize| NewtypeField {
                    ident: FieldIdent::named(fields.fields[i].0.name.clone()),
                    ty: fields.fields[i].0.ty.clone(),
                };

                Ok(NewtypeStruct {
                    sized: mk(sized),
                    zsts: zsts.into_iter().map(mk).collect(),
                })
            }
        }
    }

    /// Partitions fields into 1 sized field and an arbitrary amount of ZST fields
    ///
    /// Returns the indices to these fields
    fn partition_fields<'a>(
        attrs: impl Iterator<Item = &'a [venial::Attribute]>,
        context: impl ToTokens,
    ) -> ParseResult<(usize, Vec<usize>)> {
        let mut sized = None;
        let mut zsts = vec![];

        for (i, attr) in attrs.enumerate() {
            match KvParser::parse(attr, "godot")? {
                Some(mut parser) => {
                    if parser.handle_alone("skip")? {
                        zsts.push(i)
                    }
                    parser.finish()?;
                    // If we don't see "skip", assume its meant for someone else to handle
                }
                None if sized.is_none() => sized = Some(i),
                None => {
                    return bail!(
                        &context,
                        "GodotConvert expects a struct with a single unskipped field, found multple"
                    );
                }
            }
        }

        let Some(sized) = sized else {
            return bail!(
                &context,
                "GodotConvert expects a struct with a single sized field, found none"
            );
        };

        Ok((sized, zsts))
    }
}
