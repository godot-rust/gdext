/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::fmt::Write;

use proc_macro2::{Ident, Span, TokenStream, TokenTree};
use quote::{quote, ToTokens};

use crate::util::bail;
use crate::ParseResult;

/// Stores info from C-style enums for use in deriving `GodotConvert` and other related traits.
#[derive(Clone, Debug)]
pub struct CStyleEnum {
    /// The names of each enumerator.
    enumerator_names: Vec<Ident>,

    /// The discriminants of each variant, both explicit and implicit.
    ///
    /// Can be simple or complex expressions, the latter with parentheses:
    /// - `13`
    /// - `(1 + 2)`
    /// - `Enum::Variant as isize`
    enumerator_ords: Vec<TokenStream>,
}

impl CStyleEnum {
    /// Parses the enum.
    ///
    /// Ensures all the variants are unit variants, and that any explicit discriminants are integer literals.
    pub fn parse_enum(enum_: &venial::Enum) -> ParseResult<Self> {
        let variants = enum_
            .variants
            .items()
            .map(CStyleEnumerator::parse_enum_variant)
            .collect::<ParseResult<Vec<_>>>()?;

        let (names, ord_exprs) = Self::create_discriminant_mapping(variants)?;

        Ok(Self {
            enumerator_names: names,
            enumerator_ords: ord_exprs,
        })
    }

    fn create_discriminant_mapping(
        enumerators: Vec<CStyleEnumerator>,
    ) -> ParseResult<(Vec<Ident>, Vec<TokenStream>)> {
        // See here for how implicit discriminants are decided:
        // https://doc.rust-lang.org/reference/items/enumerations.html#implicit-discriminants
        let mut names = Vec::new();
        let mut ord_exprs = Vec::new();

        let mut last_ord = None;
        for enumerator in enumerators.into_iter() {
            let span = enumerator.discriminant_span();
            let ord = match enumerator.discriminant {
                Some(mut discriminant) => {
                    discriminant.set_span(span);
                    discriminant.to_token_stream()
                }
                None if last_ord.is_none() => quote! { 0 },
                None => quote! { #last_ord + 1 },
            };

            last_ord = Some(ord.clone());

            // let discriminant_span = enumerator.discriminant_span();
            // discriminant.set_span(discriminant_span);

            names.push(enumerator.name);
            ord_exprs.push(ord)
        }

        Ok((names, ord_exprs))
    }

    /// Returns the names of the enumerators, in order of declaration.
    pub fn enumerator_names(&self) -> &[Ident] {
        &self.enumerator_names
    }

    /// Returns the ordinal expression (discriminant) of each enumerator, in order of declaration.
    pub fn enumerator_ord_exprs(&self) -> &[TokenStream] {
        &self.enumerator_ords
    }

    /// Return a hint string for use with `PropertyHint::ENUM` where each variant has an explicit integer hint.
    pub fn to_int_hint(&self) -> TokenStream {
        // We can't build the format string directly, since the ords may be expressions and not literals.
        // Thus generate code containing a format!() statement.

        let iter = self
            .enumerator_names
            .iter()
            .zip(self.enumerator_ords.iter());

        let mut fmt = String::new();
        let mut fmt_args = Vec::new();
        let mut first = true;

        for (name, discrim) in iter {
            if first {
                first = false;
            } else {
                fmt.push(',');
            }

            write!(fmt, "{name}:{{}}").expect("write to string");
            fmt_args.push(discrim.clone());
        }

        quote! {
            format!(#fmt, #(#fmt_args),*)
        }
    }

    /// Return a hint string for use with `PropertyHint::ENUM` where the variants are just kept as strings.
    pub fn to_string_hint(&self) -> TokenStream {
        let hint_string = self
            .enumerator_names
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(",");

        hint_string.to_token_stream()
    }
}

/// Each variant in a c-style enum.
#[derive(Clone, Debug)]
pub struct CStyleEnumerator {
    /// The name of the variant.
    name: Ident,
    /// The explicit discriminant of the variant, `None` means there was no explicit discriminant.
    discriminant: Option<TokenTree>,
}

impl CStyleEnumerator {
    /// Parse an enum variant, erroring if it isn't a unit variant.
    fn parse_enum_variant(enum_variant: &venial::EnumVariant) -> ParseResult<Self> {
        match enum_variant.fields {
            venial::Fields::Unit => {}
            _ => {
                return bail!(
                    &enum_variant.fields,
                    "GodotConvert only supports C-style enums"
                )
            }
        }

        Ok(Self {
            name: enum_variant.name.clone(),
            discriminant: enum_variant.value.as_ref().map(|val| &val.value).cloned(),
        })
    }

    /// Returns a span suitable for the discriminant of the variant.
    ///
    /// If there was no explicit discriminant, this will use the span of the name instead.
    fn discriminant_span(&self) -> Span {
        match &self.discriminant {
            Some(discriminant) => discriminant.span(),
            None => self.name.span(),
        }
    }
}
