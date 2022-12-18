/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// TODO remove this warning once impl is complete
#![allow(dead_code)]
#![allow(clippy::question_mark)] // in #[derive(DeJson)]

use crate::{godot_exe, StopWatch};

use nanoserde::DeJson;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// JSON models

#[derive(DeJson)]
pub struct ExtensionApi {
    pub builtin_class_sizes: Vec<ClassSizes>,
    pub builtin_classes: Vec<BuiltinClass>,
    pub classes: Vec<Class>,
    pub global_enums: Vec<GlobalEnum>,
    pub utility_functions: Vec<UtilityFunction>,
    pub singletons: Vec<Singleton>,
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
    pub constructors: Vec<Constructor>,
    pub has_destructor: bool,
    pub operators: Vec<Operator>,
}

#[derive(DeJson)]
pub struct Operator {
    pub name: String,
    pub right_type: Option<String>, // null if unary
    pub return_type: String,
}

#[derive(DeJson)]
pub struct Class {
    pub name: String,
    pub is_refcounted: bool,
    pub is_instantiable: bool,
    pub inherits: Option<String>,
    // pub api_type: String,
    // pub constants: Option<Vec<Constant>>,
    pub enums: Option<Vec<ClassEnum>>,
    pub methods: Option<Vec<Method>>,
    // pub properties: Option<Vec<Property>>,
    // pub signals: Option<Vec<Signal>>,
}

#[derive(DeJson)]
pub struct Singleton {
    pub name: String,
    // Note: `type` currently has always same value as `name`, thus redundant
    // #[nserde(rename = "type")]
    // type_: String,
}

#[derive(DeJson)]
pub struct ClassEnum {
    pub name: String,
    pub is_bitfield: bool,
    pub values: Vec<Constant>,
}

// Same as above, but no bitfield
#[derive(DeJson)]
pub struct GlobalEnum {
    pub name: String,
    pub values: Vec<Constant>,
}

#[derive(DeJson)]
pub struct Constant {
    pub name: String,
    pub value: i32,
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
pub struct Method {
    pub name: String,
    pub is_const: bool,
    pub is_vararg: bool,
    //pub is_static: bool,
    pub is_virtual: bool,
    pub hash: Option<i64>,
    pub arguments: Option<Vec<MethodArg>>,
    pub return_value: Option<MethodReturn>,
}

impl Method {
    pub fn map_args<R>(&self, f: impl FnOnce(&Vec<MethodArg>) -> R) -> R {
        match self.arguments.as_ref() {
            Some(args) => f(args),
            None => f(&vec![]),
        }
    }
}

#[derive(DeJson, Clone)]
pub struct MethodArg {
    pub name: String,
    #[nserde(rename = "type")]
    pub type_: String,
}

#[derive(DeJson)]
pub struct MethodReturn {
    #[nserde(rename = "type")]
    pub type_: String,
}

pub trait Enum {
    fn name(&self) -> &str;
    fn values(&self) -> &Vec<Constant>;
    fn is_bitfield(&self) -> bool;
}

impl Enum for ClassEnum {
    fn name(&self) -> &str {
        &self.name
    }

    fn values(&self) -> &Vec<Constant> {
        &self.values
    }

    fn is_bitfield(&self) -> bool {
        self.is_bitfield
    }
}

impl Enum for GlobalEnum {
    fn name(&self) -> &str {
        &self.name
    }

    fn values(&self) -> &Vec<Constant> {
        &self.values
    }

    fn is_bitfield(&self) -> bool {
        // Hack until this is exported in the JSON
        self.name.contains("Flag")
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Implementation

pub fn load_extension_api(watch: &mut StopWatch) -> (ExtensionApi, &'static str) {
    // For float/double inference, see:
    // * https://github.com/godotengine/godot-proposals/issues/892
    // * https://github.com/godotengine/godot-cpp/pull/728
    let build_config = "float_64"; // TODO infer this

    let json: String = godot_exe::load_extension_api_json(watch);

    let model: ExtensionApi = DeJson::deserialize_json(&json).expect("failed to deserialize JSON");
    watch.record("deserialize_json");

    (model, build_config)
}
