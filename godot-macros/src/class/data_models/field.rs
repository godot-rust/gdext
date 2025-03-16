/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::class::{FieldExport, FieldVar};
use crate::util::error;
use proc_macro2::{Ident, Span, TokenStream};
use quote::ToTokens;

pub struct Field {
    pub name: Ident,
    pub ty: venial::TypeExpr,
    pub default_val: Option<FieldDefault>,
    pub var: Option<FieldVar>,
    pub export: Option<FieldExport>,
    pub is_onready: bool,
    pub is_oneditor: bool,
    #[cfg(feature = "register-docs")]
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
            is_oneditor: false,
            #[cfg(feature = "register-docs")]
            attributes: field.attributes.clone(),
            span: field.span(),
        }
    }

    #[must_use]
    pub fn ensure_preconditions(
        &self,
        cond: Option<FieldCond>,
        span: Span,
        errors: &mut Vec<venial::Error>,
    ) -> bool {
        let prev_size = errors.len();

        if self.default_val.is_some() {
            errors.push(error!(
                span,
                "#[init] can have at most one key among `val|node|load`"
            ));
        }

        match cond {
            Some(FieldCond::IsOnReady) if !self.is_onready => {
                errors.push(error!(
                    span,
                    "used #[init(…)] pattern requires field type `OnReady<T>`"
                ));
            }

            Some(FieldCond::IsOnEditor) if !self.is_oneditor => {
                errors.push(error!(
                    span,
                    "used #[init(…)] pattern requires field type `OnEditor<T>`"
                ));
            }

            _ => {}
        }

        errors.len() == prev_size
    }
}

pub enum FieldCond {
    IsOnReady,
    IsOnEditor,
}

pub struct Fields {
    /// All fields except `base_field`.
    pub all_fields: Vec<Field>,

    /// The field with type `Base<T>`, if available.
    pub base_field: Option<Field>,

    /// Deprecation warnings.
    pub deprecations: Vec<TokenStream>,

    /// Errors during macro evaluation that shouldn't abort the execution of the macro.
    pub errors: Vec<venial::Error>,
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
