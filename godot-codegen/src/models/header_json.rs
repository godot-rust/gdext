/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// This file acts as deserialization check of the JSON file. Even if some fields are unused, having them declared makes sure they're
// deserializable and conform to our expectations. It also doesn't add much value to annotate individual fields; it doesn't really
// matter if some are unused because it's external input data.
#![allow(dead_code, clippy::question_mark)]
// TODO(v0.6): move `dead_code` to individual fields, try to use as many fields as possible.

use nanoserde::DeJson;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// JSON models for gdextension_interface.json

#[derive(DeJson)]
pub struct HeaderJson {
    pub _copyright: Vec<String>,
    #[nserde(rename = "$schema")]
    #[expect(dead_code)]
    pub schema: String,
    pub format_version: u32,
    pub types: Vec<HeaderType>,
    pub interface: Vec<HeaderInterfaceFunction>,
}

/// A "type" in the JSON: enum, handle (pointer), alias, struct or function.
#[derive(DeJson)]
pub struct HeaderType {
    pub name: String,
    pub kind: String,
    pub description: Option<Vec<String>>,
    pub deprecated: Option<HeaderDeprecated>,

    // Enum fields.
    pub is_bitfield: Option<bool>,
    pub values: Option<Vec<HeaderEnumValue>>,

    // Handle fields.
    pub parent: Option<String>,
    pub is_const: Option<bool>,
    pub is_uninitialized: Option<bool>,

    // Alias fields.
    #[nserde(rename = "type")]
    pub type_: Option<String>,

    // Struct fields.
    pub members: Option<Vec<HeaderStructMember>>,

    // Function fields.
    pub return_value: Option<HeaderReturnValue>,
    pub arguments: Option<Vec<HeaderArgument>>,
}

// Same repr as JsonEnumConstant
#[derive(DeJson, Clone)]
pub struct HeaderEnumValue {
    pub name: String,
    pub value: i64,
    pub description: Option<Vec<String>>,
}

#[derive(DeJson)]
pub struct HeaderStructMember {
    pub name: String,
    #[nserde(rename = "type")]
    pub type_: String,
    pub description: Option<Vec<String>>,
}

// Same repr as JsonMethodReturn
#[derive(DeJson, Clone)]
pub struct HeaderReturnValue {
    #[nserde(rename = "type")]
    pub type_: String,
    pub description: Option<Vec<String>>,
}

// Same repr as JsonMethodArg
#[derive(DeJson, Clone)]
pub struct HeaderArgument {
    #[nserde(rename = "type")]
    pub type_: String,
    pub name: Option<String>,
    pub description: Option<Vec<String>>,
}

#[derive(DeJson)]
pub struct HeaderInterfaceFunction {
    pub name: String,
    pub return_value: Option<HeaderReturnValue>,
    pub arguments: Vec<HeaderArgument>,
    pub description: Vec<String>,
    pub since: String,
    pub deprecated: Option<HeaderDeprecated>,
    pub see: Option<Vec<String>>,
    pub legacy_type_name: Option<String>,
}

#[derive(DeJson)]
pub struct HeaderDeprecated {
    pub since: String,
    pub message: Option<String>,
    pub replace_with: Option<String>,
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Tests

#[cfg(test)] #[cfg_attr(published_docs, doc(cfg(test)))]
mod tests {
    use super::*;

    fn find_named<'a, T>(items: &'a [T], name: &str, get_name: impl Fn(&T) -> &str) -> &'a T {
        items
            .iter()
            .find(|item| get_name(item) == name)
            .unwrap_or_else(|| panic!("{name} not found"))
    }

    #[test]
    fn test_parse_header_json() {
        let mut watch = godot_bindings::StopWatch::start();
        let json_str = godot_bindings::load_gdextension_interface_json(&mut watch);
        let model: HeaderJson =
            DeJson::deserialize_json(json_str.as_ref()).expect("failed to deserialize JSON");

        assert_eq!(model.format_version, 1);

        let variant_type = find_named(&model.types, "GDExtensionVariantType", |t| &t.name);
        assert_eq!(variant_type.kind, "enum");
        assert!(variant_type.values.is_some());

        let call_error = find_named(&model.types, "GDExtensionCallError", |t| &t.name);
        assert_eq!(call_error.kind, "struct");
        assert!(call_error.members.is_some());

        let mem_alloc = find_named(&model.interface, "mem_alloc", |f| &f.name);
        assert_eq!(mem_alloc.since, "4.1");
    }
}
