/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// TODO remove this warning once impl is complete
#![allow(dead_code)]
#![allow(clippy::question_mark)] // in #[derive(DeJson)]

use nanoserde::DeJson;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// JSON models

#[derive(DeJson)]
pub struct ExtensionApi {
    pub header: Header,
    pub builtin_class_sizes: Vec<ClassSizes>,
    pub builtin_classes: Vec<BuiltinClass>,
    pub classes: Vec<Class>,
    pub global_enums: Vec<Enum>,
    pub utility_functions: Vec<UtilityFunction>,
    pub native_structures: Vec<NativeStructure>,
    pub singletons: Vec<Singleton>,
}

#[derive(DeJson, Clone, Debug)]
pub struct Header {
    pub version_major: u8,
    pub version_minor: u8,
    pub version_patch: u8,
    pub version_status: String,
    pub version_build: String,
    pub version_full_name: String,
}

#[derive(DeJson)]
pub struct ClassSizes {
    pub build_configuration: String,
    pub sizes: Vec<ClassSize>,
}

#[derive(DeJson)]
pub struct ClassSize {
    pub name: String,
    pub size: usize,
}

#[derive(DeJson)]
pub struct BuiltinClass {
    pub name: String,
    pub indexing_return_type: Option<String>,
    pub is_keyed: bool,
    pub members: Option<Vec<Member>>,
    // pub constants: Option<Vec<BuiltinConstant>>,
    pub enums: Option<Vec<BuiltinClassEnum>>, // no bitfield
    pub operators: Vec<Operator>,
    pub methods: Option<Vec<BuiltinClassMethod>>,
    pub constructors: Vec<Constructor>,
    pub has_destructor: bool,
}

#[derive(DeJson)]
pub struct Class {
    pub name: String,
    pub is_refcounted: bool,
    pub is_instantiable: bool,
    pub inherits: Option<String>,
    // pub api_type: String,
    pub constants: Option<Vec<ClassConstant>>,
    pub enums: Option<Vec<Enum>>,
    pub methods: Option<Vec<ClassMethod>>,
    // pub properties: Option<Vec<Property>>,
    // pub signals: Option<Vec<Signal>>,
}

#[derive(DeJson)]
pub struct NativeStructure {
    pub name: String,
    pub format: String,
}

#[derive(DeJson)]
pub struct Singleton {
    pub name: String,
    // Note: `type` currently has always same value as `name`, thus redundant
    // #[nserde(rename = "type")]
    // type_: String,
}

#[derive(DeJson)]
pub struct Enum {
    pub name: String,
    pub is_bitfield: bool,
    pub values: Vec<EnumConstant>,
}

#[derive(DeJson)]
pub struct BuiltinClassEnum {
    pub name: String,
    pub values: Vec<EnumConstant>,
}

impl BuiltinClassEnum {
    pub(crate) fn to_enum(&self) -> Enum {
        Enum {
            name: self.name.clone(),
            is_bitfield: false,
            values: self.values.clone(),
        }
    }
}

#[derive(DeJson, Clone)]
pub struct EnumConstant {
    pub name: String,
    pub value: i32,
}

pub type ClassConstant = EnumConstant;

/*
// Constants of builtin types have a string value like "Vector2(1, 1)", hence also a type field
#[derive(DeJson)]
pub struct BuiltinConstant {
    pub name: String,
    #[nserde(rename = "type")]
    pub type_: String,
    pub value: String,
}
*/

#[derive(DeJson)]
pub struct Operator {
    pub name: String,
    pub right_type: Option<String>, // null if unary
    pub return_type: String,
}

#[derive(DeJson)]
pub struct Member {
    pub name: String,
    #[nserde(rename = "type")]
    pub type_: String,
}

#[derive(DeJson)]
pub struct Property {
    #[nserde(rename = "type")]
    type_: String,
    name: String,
    setter: String,
    getter: String,
    index: i32, // can be -1
}

#[derive(DeJson)]
pub struct Signal {
    name: String,
    arguments: Option<Vec<MethodArg>>,
}

#[derive(DeJson)]
pub struct Constructor {
    pub index: usize,
    pub arguments: Option<Vec<MethodArg>>,
}

#[derive(DeJson)]
pub struct UtilityFunction {
    pub name: String,
    pub return_type: Option<String>,
    pub category: String,
    pub is_vararg: bool,
    pub hash: i64,
    pub arguments: Option<Vec<MethodArg>>,
}

#[derive(DeJson)]
pub struct BuiltinClassMethod {
    pub name: String,
    pub return_type: Option<String>,
    pub is_vararg: bool,
    pub is_const: bool,
    pub is_static: bool,
    pub hash: Option<i64>,
    pub arguments: Option<Vec<MethodArg>>,
}

#[derive(DeJson, Clone)]
pub struct ClassMethod {
    pub name: String,
    pub is_const: bool,
    pub is_vararg: bool,
    pub is_static: bool,
    pub is_virtual: bool,
    pub hash: Option<i64>,
    pub return_value: Option<MethodReturn>,
    pub arguments: Option<Vec<MethodArg>>,
}

impl ClassMethod {
    pub fn map_args<R>(&self, f: impl FnOnce(&Vec<MethodArg>) -> R) -> R {
        match self.arguments.as_ref() {
            Some(args) => f(args),
            None => f(&vec![]),
        }
    }
}

// Example: set_point_weight_scale ->
// [ {name: "id", type: "int", meta: "int64"},
//   {name: "weight_scale", type: "float", meta: "float"},
#[derive(DeJson, Clone)]
pub struct MethodArg {
    pub name: String,
    #[nserde(rename = "type")]
    pub type_: String,
    // pub meta: Option<String>,
}

// Example: get_available_point_id -> {type: "int", meta: "int64"}
#[derive(DeJson, Clone)]
pub struct MethodReturn {
    #[nserde(rename = "type")]
    pub type_: String,
    // pub meta: Option<String>,
}

impl MethodReturn {
    pub fn from_type(type_: &str) -> Self {
        Self {
            type_: type_.to_owned(),
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Implementation

pub fn load_extension_api(watch: &mut godot_bindings::StopWatch) -> (ExtensionApi, &'static str) {
    // For float/double inference, see:
    // * https://github.com/godotengine/godot-proposals/issues/892
    // * https://github.com/godotengine/godot-cpp/pull/728
    #[cfg(feature = "double-precision")]
    let build_config = "double_64"; // TODO infer this
    #[cfg(not(feature = "double-precision"))]
    let build_config = "float_64"; // TODO infer this

    // Use type inference, so we can accept both String (dynamically resolved) and &str (prebuilt).
    // #[allow]: as_ref() acts as impl AsRef<str>, but with conditional compilation

    let json = godot_bindings::load_gdextension_json(watch);
    #[allow(clippy::useless_asref)]
    let json_str: &str = json.as_ref();

    let model: ExtensionApi =
        DeJson::deserialize_json(json_str).expect("failed to deserialize JSON");
    watch.record("deserialize_json");

    println!("Parsed extension_api.json for version {:?}", model.header);

    (model, build_config)
}
