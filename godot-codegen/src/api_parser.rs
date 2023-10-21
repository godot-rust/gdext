/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// TODO remove this warning once impl is complete
#![allow(dead_code)]
#![allow(clippy::question_mark)] // in #[derive(DeJson)]

use std::collections::HashSet;

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
    pub api_type: String,
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
    pub meta: Option<String>,
    pub default_value: Option<String>,
}

// Example: get_available_point_id -> {type: "int", meta: "int64"}
#[derive(DeJson, Clone)]
pub struct MethodReturn {
    #[nserde(rename = "type")]
    pub type_: String,
    pub meta: Option<String>,
}

impl MethodReturn {
    pub fn from_type_no_meta(type_: &str) -> Self {
        Self {
            type_: type_.to_owned(),
            meta: None,
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Implementation

pub fn load_extension_api(
    watch: &mut godot_bindings::StopWatch,
) -> (ExtensionApi, [&'static str; 2]) {
    // For float/double inference, see:
    // * https://github.com/godotengine/godot-proposals/issues/892
    // * https://github.com/godotengine/godot-cpp/pull/728
    // Have to do target_pointer_width check after code generation
    // So pass a [32bit, 64bit] around of appropriate precision
    // For why see: https://github.com/rust-lang/rust/issues/42587
    let build_config: [&'static str; 2] = {
        if cfg!(feature = "double-precision") {
            ["double_32", "double_64"]
        } else {
            ["float_32", "float_64"]
        }
    };
    // Use type inference, so we can accept both String (dynamically resolved) and &str (prebuilt).
    // #[allow]: as_ref() acts as impl AsRef<str>, but with conditional compilation

    let json = godot_bindings::load_gdextension_json(watch);
    #[allow(clippy::useless_asref)]
    let json_str: &str = json.as_ref();

    let mut model: ExtensionApi =
        DeJson::deserialize_json(json_str).expect("failed to deserialize JSON");
    watch.record("deserialize_json");

    println!("Parsed extension_api.json for version {:?}", model.header);

    let used_class_names = option_env!("RUST_GDEXT_USED_CLASS_NAMES");
    if let Some(used_class_names) = used_class_names {
        let filter = HashSet::from_iter(
            used_class_names
                .split(",")
                .map(|name| name.trim().to_string()),
        );

        filter_class_names(&mut model, filter);
    } else {
        println!("Not filtering extension_api.json");
    }

    (model, build_config)
}

const MINIMAL_CLASSES: [&'static str; 27] = [
    "Engine",
    "EditorPlugin",
    "ResourceLoader",
    "AudioStreamPlayer",
    "AudioStreamPlayerVirtual",
    "Camera2D",
    "Camera2DVirtual",
    "Camera3D",
    "Camera3DVirtual",
    "Input",
    "Node",
    "Node2D",
    "Node2DVirtual",
    "Node3D",
    "Node3DVirtual",
    "NodeVirtual",
    "Object",
    "ObjectVirtual",
    "PackedScene",
    "PackedSceneExt",
    "PackedSceneVirtual",
    "RefCounted",
    "RefCountedVirtual",
    "Resource",
    "ResourceVirtual",
    "SceneTree",
    "SceneTreeVirtual",
];

fn filter_class_names(model: &mut ExtensionApi, mut allowed_class_names: HashSet<String>) {
    println!(
        "User asked to filer extension_api.json to only include the classes {:?}",
        allowed_class_names
    );

    allowed_class_names.extend(MINIMAL_CLASSES.iter().map(|x| x.to_string()));

    let all_class_names: Vec<_> = model.classes.iter().map(|class| &class.name).collect();

    let mut new_class_names = Vec::new();
    new_class_names.extend(allowed_class_names.iter().map(|name| name.clone()));
    while !new_class_names.is_empty() {
        let new_class_name = new_class_names.pop().unwrap();
        for class in model.classes.iter() {
            if class.name != new_class_name {
                continue;
            }
            if let Some(parent) = class.inherits.clone() {
                if allowed_class_names.insert(parent.clone()) {
                    new_class_names.push(parent);
                }
            }
            if let Some(methods) = &class.methods {
                for method in methods.iter() {
                    if let Some(arguments) = &method.arguments {
                        for argument in arguments.iter() {
                            for class_name in all_class_names.iter() {
                                if argument.type_.contains(*class_name) {
                                    if allowed_class_names.insert((*class_name).clone()) {
                                        new_class_names.push((*class_name).clone());
                                    }
                                }
                            }
                        }
                    }
                    if let Some(return_value) = &method.return_value {
                        for class_name in all_class_names.iter() {
                            if return_value.type_.contains(*class_name) {
                                if allowed_class_names.insert((*class_name).clone()) {
                                    new_class_names.push((*class_name).clone());
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    println!(
        "Filtering extension_api.json to only include the classes {:?}",
        allowed_class_names
    );

    let mut classes = vec![];
    std::mem::swap(&mut classes, &mut model.classes);
    model.classes.extend(
        classes
            .into_iter()
            .filter(|class: &Class| allowed_class_names.contains(&class.name as &str)),
    );
}
