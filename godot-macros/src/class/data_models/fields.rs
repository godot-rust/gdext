/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::class::Field;
use crate::util::bail;
use crate::ParseResult;
use proc_macro2::{Punct, TokenStream};
use std::fmt::Display;

pub struct Fields {
    /// Names of all the declared groups and subgroups for this struct.
    // In the future might be split in two (for groups and subgroups) & used to define the priority (order) of said groups.
    // Currently order of declaration declares the group priority (i.e. – groups declared first are shown as the first in the editor).
    // This order is not guaranteed but so far proved to work reliably.
    pub groups: Vec<String>,

    /// All fields except `base_field`.
    pub all_fields: Vec<Field>,

    /// The field with type `Base<T>`, if available.
    pub base_field: Option<Field>,

    /// Deprecation warnings.
    pub deprecations: Vec<TokenStream>,

    /// Errors during macro evaluation that shouldn't abort the execution of the macro.
    pub errors: Vec<venial::Error>,
}

impl Fields {
    /// Remove surrounding quotes to display declared "group name" in editor as `group name` instead of `"group name"`.
    /// Should be called after parsing all the fields to avoid unnecessary operations.
    pub fn format_groups(&mut self) {
        self.groups
            .iter_mut()
            .for_each(|g| *g = g.trim_matches('"').to_string());
    }
}

/// Fetches data for all named fields for a struct.
///
/// Errors if `class` is a tuple struct.
pub fn named_fields(
    class: &venial::Struct,
    derive_macro_name: impl Display,
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
