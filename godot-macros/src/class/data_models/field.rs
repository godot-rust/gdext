/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::class::{FieldExport, FieldVar};
use proc_macro2::{Ident, Span, TokenStream};
use quote::ToTokens;
use venial::Error;

pub struct Field {
    pub name: Ident,
    pub ty: venial::TypeExpr,
    pub default_val: Option<FieldDefault>,
    pub var: Option<FieldVar>,
    pub export: Option<FieldExport>,
    pub is_onready: bool,
    #[cfg(feature = "docs")]
    pub attributes: Vec<venial::Attribute>,
    pub span: Span,
}

impl Field {
    pub fn new(field: &venial::NamedField) -> Self {
        Self {
            name: field.name.clone(),
            ty: field.ty.clone(),
            default_val: None,
            var: None,
            export: None,
            is_onready: false,
            #[cfg(feature = "docs")]
            attributes: field.attributes.clone(),
            span: field.span(),
        }
    }
}

pub struct Fields {
    /// All fields except `base_field`.
    pub all_fields: Vec<Field>,

    /// The field with type `Base<T>`, if available.
    pub base_field: Option<Field>,

    /// The base field is either absent or is correctly formatted.
    ///
    /// When this is false, there will always be a compile error ensuring the program fails to compile.
    pub well_formed_base: bool,

    /// Deprecation warnings.
    pub deprecations: Vec<TokenStream>,

    /// Errors during macro evaluation that shouldn't abort the execution of the macro.
    pub errors: Vec<Error>,
}

#[derive(Clone)]
pub struct FieldDefault {
    pub default_val: TokenStream,
    pub span: Span,
}

impl ToTokens for FieldDefault {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.default_val.to_tokens(tokens)
    }
}
