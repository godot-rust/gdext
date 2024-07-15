/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use proc_macro2::{Ident, TokenStream};
use quote::quote;

use crate::util::bail;
use crate::ParseResult;

/// Stores info from the field of a newtype struct for use in deriving `GodotConvert` and other related traits.
pub struct NewtypeStruct {
    /// The name of the field.
    ///
    /// If `None`, then this represents a tuple-struct with one field.
    pub name: Option<Ident>,

    /// The type of the field.
    pub ty: venial::TypeExpr,
}

impl NewtypeStruct {
    /// Parses a struct into a newtype struct.
    ///
    /// This will fail if the struct doesn't have exactly one field.
    pub fn parse_struct(struct_: &venial::Struct) -> ParseResult<NewtypeStruct> {
        match &struct_.fields {
            venial::Fields::Unit => bail!(&struct_.fields, "GodotConvert expects a struct with a single field, unit structs are currently not supported"),
            venial::Fields::Tuple(fields) => {
                if fields.fields.len() != 1 {
                    return bail!(&fields.fields, "GodotConvert expects a struct with a single field, not {} fields", fields.fields.len())
                }

                let (field, _) = fields.fields[0].clone();

                Ok(NewtypeStruct { name: None, ty: field.ty })
            },
            venial::Fields::Named(fields) => {
                if fields.fields.len() != 1 {
                    return bail!(&fields.fields, "GodotConvert expects a struct with a single field, not {} fields", fields.fields.len())
                }

                let (field, _) = fields.fields[0].clone();

                Ok(NewtypeStruct { name: Some(field.name), ty: field.ty })
            },
        }
    }

    /// Gets the field name.
    ///
    /// If this represents a tuple-struct, then it will return `0`. This can be used just like it was a named field with the name `0`.
    /// For instance:
    /// ```
    /// struct Foo(i64);
    ///
    /// let mut foo = Foo { 0: 10 };
    /// foo.0 = 20;
    /// println!("{}", foo.0);
    /// ```
    pub fn field_name(&self) -> TokenStream {
        match &self.name {
            Some(name) => quote! { #name },
            None => quote! { 0 },
        }
    }
}
