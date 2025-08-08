/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use proc_macro2::{Punct, TokenStream};

use crate::class::Field;
use crate::util::bail;
use crate::ParseResult;

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

/// Fetches data for all named fields for a struct.
///
/// Errors if `class` is a tuple struct.
pub fn named_fields(
    class: &venial::Struct,
    derive_macro_name: &str,
) -> ParseResult<Vec<(venial::NamedField, Punct)>> {
    // This is separate from parse_fields to improve compile errors. The errors from here demand larger and more non-local changes from the API
    // user than those from parse_struct_attributes, so this must be run first.
    match &class.fields {
        // TODO disallow unit structs in the future
        // It often happens that over time, a registered class starts to require a base field.
        // Extending a {} struct requires breaking less code, so we should encourage it from the start.
        venial::Fields::Unit => Ok(vec![]),
        venial::Fields::Tuple(_) => bail!(
            &class.fields,
            "{derive_macro_name} is not supported for tuple structs",
        )?,
        venial::Fields::Named(fields) => Ok(fields.fields.inner.clone()),
    }
}
