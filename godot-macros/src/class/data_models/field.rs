/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::class::{FieldExport, FieldVar};
use proc_macro2::{Ident, TokenStream};

pub struct Field {
    pub name: Ident,
    pub ty: venial::TypeExpr,
    pub default_val: Option<TokenStream>,
    pub var: Option<FieldVar>,
    pub export: Option<FieldExport>,
    pub is_onready: bool,
    #[cfg(feature = "register-docs")]
    pub attributes: Vec<venial::Attribute>,
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
            #[cfg(feature = "register-docs")]
            attributes: field.attributes.clone(),
        }
    }
}

pub struct Fields {
    /// All fields except `base_field`.
    pub all_fields: Vec<Field>,

    /// The field with type `Base<T>`, if available.
    pub base_field: Option<Field>,

    /// Deprecation warnings.
    pub deprecations: Vec<TokenStream>,
}
