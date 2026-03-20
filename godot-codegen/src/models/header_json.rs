/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// This file acts as deserialization check of the JSON file. Even if some fields are unused, having them declared makes sure they're
// deserializable and conform to our expectations. It also doesn't add much value to annotate individual fields; it doesn't really
// matter if some are unused because it's external input data.
// In #[derive(DeJson)]: "this block may be rewritten with the `?` operator"
#![allow(clippy::question_mark)] // <- CHECK IF STILL NEEDED
#![allow(dead_code)]

use nanoserde::DeJson;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// JSON models for gdextension_interface.json

#[derive(DeJson)]
pub struct HeaderJson {
    pub _copyright: Vec<String>,
    #[nserde(rename = "$schema")]
    pub schema: String,
    pub format_version: u32,
    pub types: Vec<HeaderType>,
    pub interface: Vec<HeaderInterfaceFunction>,
}

#[derive(DeJson)]
pub struct HeaderType {
    pub name: String,
    pub kind: String,
    pub description: Option<Vec<String>>,
    pub deprecated: Option<HeaderDeprecated>,
    // enum fields
    pub values: Option<Vec<HeaderEnumValue>>,
    // handle fields
    pub parent: Option<String>,
    #[nserde(rename = "const")]
    pub is_const: Option<bool>,
    // alias fields
    #[nserde(rename = "type")]
    pub type_: Option<String>,
    // struct fields
    pub members: Option<Vec<HeaderStructMember>>,
    // function fields
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
    pub return_value: HeaderReturnValue,
    pub arguments: Vec<HeaderArgument>,
    pub description: Vec<String>,
    pub since: String,
    pub deprecated: Option<HeaderDeprecated>,
    pub see: Option<Vec<String>>,
    pub legacy_type_name: Option<String>,
}

#[derive(DeJson, Clone)]
pub struct HeaderDeprecated {
    pub since: String,
    pub message: Option<String>,
    pub replace_with: Option<String>,
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_header_json() {
        let json_str = std::fs::read_to_string("../json/gdextension_interface.json")
            .expect("failed to read JSON file");
        let model: HeaderJson =
            DeJson::deserialize_json(&json_str).expect("failed to deserialize JSON");

        // Verify format version
        assert_eq!(model.format_version, 1);

        // Verify some types exist
        assert!(model
            .types
            .iter()
            .any(|t| t.name == "GDExtensionVariantType"));
        assert!(model.types.iter().any(|t| t.name == "GDExtensionCallError"));

        // Verify interface functions exist
        assert!(!model.interface.is_empty());

        // Spot-check a specific enum type
        let variant_type = model
            .types
            .iter()
            .find(|t| t.name == "GDExtensionVariantType")
            .expect("GDExtensionVariantType not found");
        assert_eq!(variant_type.kind, "enum");
        assert!(variant_type.values.is_some());

        // Spot-check a specific struct type
        let call_error = model
            .types
            .iter()
            .find(|t| t.name == "GDExtensionCallError")
            .expect("GDExtensionCallError not found");
        assert_eq!(call_error.kind, "struct");
        assert!(call_error.members.is_some());

        // Spot-check an interface function
        let interface_fn = model
            .interface
            .iter()
            .find(|f| f.name == "variant_new_copy");
        assert!(interface_fn.is_some());
    }
}
