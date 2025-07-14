/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// Note: group membership for properties in Godot is based on the order of their registration.
// All the properties belong to group or subgroup registered beforehand, identically as in GDScript.
// Initial implementation providing clap-like API with an explicit sorting
// & groups/subgroups declared for each field (`#[export(group = ..., subgroup = ...)]`
// can be found at: https://github.com/godot-rust/gdext/pull/1214.

use crate::util::{bail, KvParser};
use crate::ParseResult;
use proc_macro2::Literal;

/// Specifies group or subgroup which starts with a given field.
/// Group membership for properties in Godot is based on the order of their registration –
/// i.e. given field belongs to group declared beforehand (for example with some previous field).
pub struct FieldGroup {
    pub(crate) name: Literal,
    pub(crate) prefix: Literal,
}

impl FieldGroup {
    pub(crate) fn new_from_kv(parser: &mut KvParser) -> ParseResult<Self> {
        let Some(name) = parser.handle_literal("name", "String")? else {
            return bail!(parser.span(), "missing required argument: `name = \"...\".");
        };

        let prefix = parser
            .handle_literal("prefix", "String")?
            .unwrap_or(Literal::string(""));

        Ok(Self { name, prefix })
    }
}
