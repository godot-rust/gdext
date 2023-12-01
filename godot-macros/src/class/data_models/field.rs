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
    pub ty: venial::TyExpr,
    pub default: Option<TokenStream>,
    pub var: Option<FieldVar>,
    pub export: Option<FieldExport>,
}

impl Field {
    pub fn new(field: &venial::NamedField) -> Self {
        Self {
            name: field.name.clone(),
            ty: field.ty.clone(),
            default: None,
            var: None,
            export: None,
        }
    }
}

pub struct Fields {
    /// All fields except `base_field`.
    pub all_fields: Vec<Field>,

    /// The field annotated with `#[base]`.
    pub base_field: Option<Field>,
}
