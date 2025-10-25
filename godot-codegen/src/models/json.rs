/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// In #[derive(DeJson)]: "this block may be rewritten with the `?` operator"
#![allow(clippy::question_mark)]

// This file acts as deserialization check of the JSON file. Even if some fields are unused, having them declared makes sure they're
// deserializable and conform to our expectations. It also doesn't add much value to annotate individual fields; it doesn't really
// matter if some are unused because it's external input data.

use nanoserde::DeJson;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// JSON models

#[derive(DeJson)]
pub struct JsonExtensionApi {
    pub header: JsonHeader,
    pub builtin_class_sizes: Vec<JsonBuiltinSizes>,
    pub builtin_classes: Vec<JsonBuiltinClass>,
    pub classes: Vec<JsonClass>,
    pub global_enums: Vec<JsonEnum>,
    pub utility_functions: Vec<JsonUtilityFunction>,
    pub native_structures: Vec<JsonNativeStructure>,
    pub singletons: Vec<JsonSingleton>,
}

#[derive(DeJson, Clone, Debug)]
pub struct JsonHeader {
    pub version_major: u8,
    pub version_minor: u8,
    pub version_patch: u8,
    #[allow(dead_code)]
    pub version_status: String,
    #[allow(dead_code)]
    pub version_build: String,
    pub version_full_name: String,
}

#[derive(DeJson)]
pub struct JsonBuiltinSizes {
    pub build_configuration: String,
    pub sizes: Vec<JsonBuiltinSizeForConfig>,
}

#[derive(DeJson)]
pub struct JsonBuiltinSizeForConfig {
    pub name: String,
    pub size: usize,
}

#[derive(DeJson)]
pub struct JsonBuiltinClass {
    pub name: String,
    #[allow(dead_code)]
    pub indexing_return_type: Option<String>,
    #[allow(dead_code)]
    pub is_keyed: bool,
    // pub members: Option<Vec<Member>>,
    // pub constants: Option<Vec<BuiltinConstant>>,
    pub enums: Option<Vec<JsonBuiltinEnum>>, // no bitfield
    pub operators: Vec<JsonOperator>,
    pub methods: Option<Vec<JsonBuiltinMethod>>,
    pub constructors: Vec<JsonConstructor>,
    pub has_destructor: bool,
}

#[derive(DeJson)]
pub struct JsonClass {
    pub name: String,
    pub is_refcounted: bool,
    pub is_instantiable: bool,
    pub inherits: Option<String>,
    pub api_type: String,
    pub constants: Option<Vec<JsonClassConstant>>,
    pub enums: Option<Vec<JsonEnum>>,
    pub methods: Option<Vec<JsonClassMethod>>,
    // pub properties: Option<Vec<Property>>,
    pub signals: Option<Vec<JsonSignal>>,
}

#[derive(DeJson)]
pub struct JsonNativeStructure {
    pub name: String,
    pub format: String,
}

#[derive(DeJson)]
pub struct JsonSingleton {
    pub name: String,
    // Note: `type` currently has always same value as `name`, thus redundant
    // #[nserde(rename = "type")]
    // type_: String,
}

#[derive(DeJson)]
pub struct JsonEnum {
    pub name: String,
    pub is_bitfield: bool,
    pub values: Vec<JsonEnumConstant>,
}

#[derive(DeJson)]
pub struct JsonBuiltinEnum {
    pub name: String,
    pub values: Vec<JsonEnumConstant>,
}

impl JsonBuiltinEnum {
    pub fn to_enum(&self) -> JsonEnum {
        JsonEnum {
            name: self.name.clone(),
            is_bitfield: false,
            values: self.values.clone(),
        }
    }
}

#[derive(DeJson, Clone)]
pub struct JsonEnumConstant {
    pub name: String,

    // i64 is common denominator for enum, bitfield and constant values.
    // Note that values > i64::MAX will be implicitly wrapped, see https://github.com/not-fl3/nanoserde/issues/89.
    pub value: i64,
}

impl JsonEnumConstant {
    pub fn to_enum_ord(&self) -> i32 {
        self.value.try_into().unwrap_or_else(|_| {
            panic!(
                "enum value {} = {} is out of range for i32, please report this",
                self.name, self.value
            )
        })
    }
}

pub type JsonClassConstant = JsonEnumConstant;

/*
// Constants of builtin types have a string value like "Vector2(1, 1)", hence also a type field
#[derive(DeJson)]
pub struct JsonBuiltinConstant {
    pub name: String,
    #[nserde(rename = "type")]
    pub type_: String,
    pub value: String,
}
*/

#[derive(DeJson)]
pub struct JsonOperator {
    pub name: String,
    #[allow(dead_code)]
    pub right_type: Option<String>, // null if unary
    #[allow(dead_code)]
    pub return_type: String,
}

#[derive(DeJson)]
#[allow(dead_code)]
pub struct JsonMember {
    pub name: String,
    #[nserde(rename = "type")]
    pub type_: String,
}

#[derive(DeJson)]
#[allow(dead_code)]
pub struct JsonProperty {
    #[nserde(rename = "type")]
    type_: String,
    name: String,
    setter: String,
    getter: String,
    index: i32, // can be -1
}

#[derive(DeJson)]
pub struct JsonSignal {
    pub name: String,
    pub arguments: Option<Vec<JsonMethodArg>>,
}

#[derive(DeJson)]
pub struct JsonConstructor {
    pub index: usize,
    pub arguments: Option<Vec<JsonMethodArg>>,
}

#[derive(DeJson)]
pub struct JsonUtilityFunction {
    pub name: String,
    pub return_type: Option<String>,
    /// Category: `"general"` or `"math"`
    #[allow(dead_code)]
    pub category: String,
    pub is_vararg: bool,
    pub hash: i64,
    pub arguments: Option<Vec<JsonMethodArg>>,
}

#[derive(DeJson)]
pub struct JsonBuiltinMethod {
    pub name: String,
    pub return_type: Option<String>,
    pub is_vararg: bool,
    pub is_const: bool,
    pub is_static: bool,
    pub hash: Option<i64>,
    pub arguments: Option<Vec<JsonMethodArg>>,
}

#[derive(DeJson, Clone)]
pub struct JsonClassMethod {
    pub name: String,
    pub is_const: bool,
    pub is_vararg: bool,
    pub is_static: bool,
    pub is_virtual: bool,
    #[cfg(since_api = "4.4")]
    pub is_required: Option<bool>, // Only virtual functions have this field.
    pub hash: Option<i64>,
    pub return_value: Option<JsonMethodReturn>,
    pub arguments: Option<Vec<JsonMethodArg>>,
}

// Example: set_point_weight_scale ->
// [ {name: "id", type: "int", meta: "int64"},
//   {name: "weight_scale", type: "float", meta: "float"},
#[derive(DeJson, Clone)]
pub struct JsonMethodArg {
    pub name: String,
    #[nserde(rename = "type")]
    pub type_: String,
    /// Extra information about the type (e.g. which integer). Value "required" indicates non-nullable class types (Godot 4.6+).
    pub meta: Option<String>,
    pub default_value: Option<String>,
}

// Example: get_available_point_id -> {type: "int", meta: "int64"}
#[derive(DeJson, Clone)]
pub struct JsonMethodReturn {
    #[nserde(rename = "type")]
    pub type_: String,
    /// Extra information about the type (e.g. which integer). Value "required" indicates non-nullable class types (Godot 4.6+).
    pub meta: Option<String>,
}

impl JsonMethodReturn {
    pub fn from_type_no_meta(type_: &str) -> Self {
        Self {
            type_: type_.to_owned(),
            meta: None,
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Implementation

pub fn load_extension_api(watch: &mut godot_bindings::StopWatch) -> JsonExtensionApi {
    // Use type inference, so we can accept both String (dynamically resolved) and &str (prebuilt).
    // #[allow]: as_ref() acts as impl AsRef<str>, but with conditional compilation

    let json = godot_bindings::load_gdextension_json(watch);
    let json_str: &str = json.as_ref();

    let model: JsonExtensionApi =
        DeJson::deserialize_json(json_str).expect("failed to deserialize JSON");
    watch.record("deserialize_json");

    println!("Parsed extension_api.json for version {:?}", model.header);
    model
}
