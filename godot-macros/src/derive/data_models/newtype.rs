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

pub enum FieldIdentifier {
    /// Index of the field
    Tuple(Literal),
    /// Name of the field
    Named(Ident),
}

pub enum FieldIdentifiers {
    /// Indices of the fields
    Tuple(Vec<Literal>),
    /// Names of the fields
    Named(Vec<Ident>),
}

/// Stores info from the field of a newtype struct for use in deriving `GodotConvert` and other related traits.
///
/// `NewtypeStruct` must have exactly 1 sized field, and can have an arbitrary amount of ZST fields.
pub struct NewtypeStruct {
    /// The identifier of the sized field.
    pub field: FieldIdentifier,

    /// The type of the sized field.
    pub ty: venial::TypeExpr,

    /// The identifiers of the ZST fields.
    pub zst_fields: FieldIdentifiers,

    /// The types of the ZST fields.
    pub zst_tys: Vec<venial::TypeExpr>,
}

// Helper trait to abstract over NamedField and TupleField.
trait Field {
    fn get_attributes(&self) -> &[venial::Attribute];
}

impl Field for (usize, &venial::TupleField) {
    fn get_attributes(&self) -> &[venial::Attribute] {
        self.1.attributes.as_slice()
    }
}

impl Field for &venial::NamedField {
    fn get_attributes(&self) -> &[venial::Attribute] {
        self.attributes.as_slice()
    }
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
                let (field, zst_fields) = Self::partition_fields(
                    fields.fields.iter().map(|(field, _)| field).enumerate(),
                    fields,
                )?;

                let (zst_names, zst_tys) = zst_fields
                    .into_iter()
                    .map(|(id, field)| (Literal::usize_unsuffixed(id), field.ty.clone()))
                    .unzip();

                Ok(NewtypeStruct {
                    field: FieldIdentifier::Tuple(Literal::usize_unsuffixed(field.0)),
                    ty: field.1.ty.clone(),
                    zst_fields: FieldIdentifiers::Tuple(zst_names),
                    zst_tys,
                })
            }
            venial::Fields::Named(fields) => {
                let (field, zst_fields) =
                    Self::partition_fields(fields.fields.iter().map(|(field, _)| field), fields)?;

                let (zst_names, zst_tys) = zst_fields
                    .into_iter()
                    .map(|field| (field.name.clone(), field.ty.clone()))
                    .unzip();

                Ok(NewtypeStruct {
                    field: FieldIdentifier::Named(field.name.clone()),
                    ty: field.ty.clone(),
                    zst_fields: FieldIdentifiers::Named(zst_names),
                    zst_tys,
                })
            }
        }
    }

    /// Partitions fields into 1 sized field and an arbitrary amount of ZST fields
    fn partition_fields<T: Field>(
        fields: impl Iterator<Item = T>,
        context: impl ToTokens,
    ) -> ParseResult<(T, Vec<T>)> {
        let mut sized_field = None;
        let mut zst_fields = vec![];

        for field in fields {
            match KvParser::parse(field.get_attributes(), "godot")? {
                Some(mut parser) => {
                    if parser.handle_alone("skip")? {
                        zst_fields.push(field)
                    }
                    // If we don't see "skip", assume its meant for someone else to handle
                }
                None => {
                    if sized_field.is_none() {
                        sized_field = Some(field);
                    } else {
                        bail!(
                            &context,
                            "GodotConvert expects a struct with a single unskipped field, found multple",
                        )?;
                    }
                }
            }
        }

        if sized_field.is_none() {
            bail!(
                &context,
                "GodotConvert expects a struct with a single sized field, found none",
            )?;
        }

        Ok((sized_field.unwrap(), zst_fields))
    }

    /// Gets the field name.
    ///
    /// If this represents a tuple-struct, then it will return a number. This can be used just like it was a named field.
    /// For instance:
    /// ```
    /// struct Foo(i64);
    ///
    /// let mut foo = Foo { 0: 10 };
    /// foo.0 = 20;
    /// println!("{}", foo.0);
    /// ```
    pub fn field_name(&self) -> TokenStream {
        match &self.field {
            FieldIdentifier::Named(name) => quote! { #name },
            FieldIdentifier::Tuple(num) => quote! { #num },
        }
    }

    /// Gets the phantom field names.
    ///
    /// If this represents a tuple-struct, then it will return numbers. See `Self::field_name`
    pub fn zst_field_names(&self) -> Vec<TokenStream> {
        match &self.zst_fields {
            FieldIdentifiers::Named(vec) => vec.iter().map(|ident| quote! {#ident}).collect(),
            FieldIdentifiers::Tuple(vec) => vec.iter().map(|ident| quote! {#ident}).collect(),
        }
    }
}
