/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use proc_macro2::{Ident, Span, TokenStream};
use quote::ToTokens;

use crate::util::{bail, KvParser};
use crate::ParseResult;

/// Stores data related to the `#[godot(...)]` attribute.
pub enum GodotAttribute {
    /// `#[godot(transparent)]`
    Transparent { span: Span },
    /// `#[godot(via = via_type)]`
    Via { span: Span, via_type: ViaType },
}

impl GodotAttribute {
    pub fn parse_attribute(item: &venial::Item) -> ParseResult<Self> {
        let mut parser = KvParser::parse_required(item.attributes(), "godot", item)?;
        let attribute = Self::parse(&mut parser)?;
        parser.finish()?;

        Ok(attribute)
    }

    fn parse(parser: &mut KvParser) -> ParseResult<Self> {
        let span = parser.span();

        if parser.handle_alone("transparent")? {
            return Ok(Self::Transparent { span });
        }

        if let Some(via_type) = parser.handle_ident("via")? {
            return Ok(Self::Via {
                span,
                via_type: ViaType::parse_ident(via_type)?,
            });
        }

        bail!(
            span,
            "expected either `#[godot(transparent)]` or `#[godot(via = <via_type>)]`"
        )
    }

    /// The span of the entire attribute.
    ///
    /// Specifically this is the span of the `[ ]` group from a `#[godot(...)]` attribute.
    pub fn span(&self) -> Span {
        match self {
            GodotAttribute::Transparent { span } => *span,
            GodotAttribute::Via { span, .. } => *span,
        }
    }
}

/// The via type from a `#[godot(via = via_type)]` attribute.
pub enum ViaType {
    /// The via type is `GString`
    GString { gstring_ident: Ident },
    /// The via type is an integer
    Int { int_ident: Ident },
}

impl ViaType {
    fn parse_ident(ident: Ident) -> ParseResult<Self> {
        let via_type = match ident.to_string().as_str() {
            "GString" => ViaType::GString { gstring_ident: ident },
            "i8" |"i16" | "i32" | "i64" | "u8" | "u16" | "u32" => ViaType::Int { int_ident: ident },
            other => return bail!(ident, "Via type `{other}` is not supported, expected one of: GString, i8, i16, i32, i64, u8, u16, u32")
        };

        Ok(via_type)
    }
}

impl ToTokens for ViaType {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            ViaType::GString { gstring_ident } => gstring_ident.to_tokens(tokens),
            ViaType::Int { int_ident } => int_ident.to_tokens(tokens),
        }
    }
}
