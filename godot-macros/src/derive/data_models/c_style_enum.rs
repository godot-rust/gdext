/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use proc_macro2::{Ident, Literal, Span, TokenTree};

use crate::util::{bail, error};
use crate::ParseResult;

/// Stores info from c-style enums for use in deriving `GodotConvert` and other related traits.
#[derive(Clone, Debug)]
pub struct CStyleEnum {
    /// The names of each variant.
    enumerator_names: Vec<Ident>,
    /// The discriminants of each variant, both explicit and implicit.
    enumerator_ords: Vec<Literal>,
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

        let (names, discriminants) = Self::create_discriminant_mapping(variants)?;

        Ok(Self {
            enumerator_names: names,
            enumerator_ords: discriminants,
        })
    }

    fn create_discriminant_mapping(
        enumerators: Vec<CStyleEnumerator>,
    ) -> ParseResult<(Vec<Ident>, Vec<Literal>)> {
        // See here for how implicit discriminants are decided
        // https://doc.rust-lang.org/reference/items/enumerations.html#implicit-discriminants
        let mut names = Vec::new();
        let mut discriminants = Vec::new();

        let mut last_discriminant = None;
        for enumerator in enumerators.into_iter() {
            let discriminant_span = enumerator.discriminant_span();

            let discriminant = match enumerator.discriminant_as_i64()? {
                Some(discriminant) => discriminant,
                None => last_discriminant.unwrap_or(0) + 1,
            };
            last_discriminant = Some(discriminant);

            let mut discriminant = Literal::i64_unsuffixed(discriminant);
            discriminant.set_span(discriminant_span);

            names.push(enumerator.name);
            discriminants.push(discriminant)
        }

        Ok((names, discriminants))
    }

    /// Returns the names of the variants, in order of the variants.
    pub fn names(&self) -> &[Ident] {
        &self.enumerator_names
    }

    /// Returns the discriminants of each variant, in order of the variants.
    pub fn discriminants(&self) -> &[Literal] {
        &self.enumerator_ords
    }

    /// Return a hint string for use with `PropertyHint::ENUM` where each variant has an explicit integer hint.
    pub fn to_int_hint(&self) -> String {
        self.enumerator_names
            .iter()
            .zip(self.enumerator_ords.iter())
            .map(|(name, discrim)| format!("{name}:{discrim}"))
            .collect::<Vec<_>>()
            .join(",")
    }

    /// Return a hint string for use with `PropertyHint::ENUM` where the variants are just kept as strings.
    pub fn to_string_hint(&self) -> String {
        self.enumerator_names
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(",")
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
        match enum_variant.contents {
            venial::StructFields::Unit => {}
            _ => {
                return bail!(
                    &enum_variant.contents,
                    "GodotConvert only supports c-style enums"
                )
            }
        }

        Ok(Self {
            name: enum_variant.name.clone(),
            discriminant: enum_variant.value.as_ref().map(|val| &val.value).cloned(),
        })
    }

    /// Returns the discriminant parsed as an i64 literal.
    fn discriminant_as_i64(&self) -> ParseResult<Option<i64>> {
        let Some(discriminant) = self.discriminant.as_ref() else {
            return Ok(None);
        };

        let int = discriminant
            .to_string()
            .parse::<i64>()
            .map_err(|_| error!(discriminant, "expected i64 literal"))?;

        Ok(Some(int))
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
