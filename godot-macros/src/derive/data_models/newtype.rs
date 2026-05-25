/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use proc_macro2::{Ident, Literal, Span, TokenStream};
use quote::quote;
use venial::{NamedField, TupleField};

use crate::ParseResult;
use crate::util::bail;

pub enum FieldType {
    Tuple(Literal),
    Named(Ident),
}

pub enum FieldsType {
    Tuple(Vec<Literal>),
    Named(Vec<Ident>),
}

/// Stores info from the field of a newtype struct for use in deriving `GodotConvert` and other related traits.
///
/// Here, a newtype struct must have exactly 1 non-ZST field, and can have an arbitrary amount of ZST fields.
pub struct NewtypeStruct {
    /// The name of the field.
    ///
    /// If `None`, then this represents a tuple-struct with one field.
    pub name: FieldType,

    /// The names of the phantom fields.
    pub phantom_names: FieldsType,

    /// The type of the field.
    pub ty: venial::TypeExpr,
}

impl NewtypeStruct {
    /// Parses a struct into a newtype struct.
    ///
    /// This will fail if the struct doesn't have exactly one field.
    pub fn parse_struct(struct_: &venial::Struct) -> ParseResult<NewtypeStruct> {
        match &struct_.fields {
            venial::Fields::Unit => bail!(
                &struct_.fields,
                "GodotConvert expects a struct with a single field, unit structs are currently not supported"
            ),
            venial::Fields::Tuple(fields) => {
                fn phantom_predicate(field: &TupleField) -> bool {
                    // Some types we don't care about are not paths, like references
                    if let Some(path) = field.ty.as_path() {
                        // This unwrap only fails if the field had no type specified, which isn't valid code anyways.
                        return path.segments.last().unwrap().ident
                            == Ident::new("PhantomData", Span::mixed_site());
                    }
                    false
                }

                let mut non_phantom_fields = fields
                    .fields
                    .items()
                    .enumerate()
                    .filter(|(_, field)| !phantom_predicate(field));

                let maybe_field = non_phantom_fields.next();

                let total_count = if maybe_field.is_none() {
                    0
                } else {
                    non_phantom_fields.count() + 1
                };

                if total_count != 1 {
                    return bail!(
                        &fields.fields,
                        "GodotConvert expects a struct with a single non-PhantomData field, not {} fields",
                        total_count
                    );
                }

                let (field_num, field) = maybe_field.unwrap();

                let phantom_nums = (0..field_num)
                    .chain(field_num + 1..fields.fields.len())
                    .map(Literal::usize_unsuffixed)
                    .collect();

                Ok(NewtypeStruct {
                    name: FieldType::Tuple(Literal::usize_unsuffixed(field_num)),
                    phantom_names: FieldsType::Tuple(phantom_nums),
                    ty: field.ty.clone(),
                })
            }
            venial::Fields::Named(fields) => {
                fn phantom_predicate(field: &NamedField) -> bool {
                    // Some types we don't care about are not paths, like references
                    if let Some(path) = field.ty.as_path() {
                        // This unwrap only fails if the field had no type specified, which isn't valid code anyways.
                        return path.segments.last().unwrap().ident
                            == Ident::new("PhantomData", Span::mixed_site());
                    }
                    false
                }

                let mut non_phantom_fields = fields
                    .fields
                    .items()
                    .filter(|field| !phantom_predicate(field));

                let maybe_field = non_phantom_fields.next();

                let total_count = if maybe_field.is_none() {
                    0
                } else {
                    non_phantom_fields.count() + 1
                };

                if total_count != 1 {
                    return bail!(
                        &fields.fields,
                        "GodotConvert expects a struct with a single non-PhantomData field, not {} fields",
                        total_count
                    );
                }

                let field = maybe_field.unwrap().clone();

                let phantom_names = fields
                    .fields
                    .items()
                    .filter_map(|field| {
                        if phantom_predicate(field) {
                            return Some(field.name.clone());
                        }
                        None
                    })
                    .collect();

                Ok(NewtypeStruct {
                    name: FieldType::Named(field.name),
                    phantom_names: FieldsType::Named(phantom_names),
                    ty: field.ty,
                })
            }
        }
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
        match &self.name {
            FieldType::Named(name) => quote! { #name },
            FieldType::Tuple(num) => quote! { #num },
        }
    }

    /// Gets the phantom field names.
    ///
    /// If this represents a tuple-struct, then it will return numbers. See `Self::field_name`
    pub fn phantom_field_names(&self) -> Vec<TokenStream> {
        match &self.phantom_names {
            FieldsType::Named(vec) => vec.iter().map(|ident| quote! {#ident}).collect(),
            FieldsType::Tuple(vec) => vec.iter().map(|ident| quote! {#ident}).collect(),
        }
    }
}
