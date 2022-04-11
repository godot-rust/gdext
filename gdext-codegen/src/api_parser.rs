use crate::godot_exe;

use miniserde::{json, Deserialize};

// ----------------------------------------------------------------------------------------------------------------------------------------------
// JSON models

#[derive(Deserialize)]
pub struct ExtensionApi {
    pub builtin_class_sizes: Vec<ClassSizes>,
    pub builtin_classes: Vec<BuiltinClass>,
    pub classes: Vec<Class>,
    pub global_enums: Vec<Enum>,
}

#[derive(Deserialize)]
pub struct ClassSizes {
    pub build_configuration: String,
    pub sizes: Vec<ClassSize>,
}

#[derive(Deserialize)]
pub struct ClassSize {
    pub name: String,
    pub size: usize,
}

#[derive(Deserialize)]
pub struct BuiltinClass {
    pub name: String,
    pub constructors: Vec<Constructor>,
    pub has_destructor: bool,
}

#[derive(Deserialize)]
pub struct Class {
    pub name: String,
    pub is_refcounted: bool,
    pub is_instantiable: bool,
    pub inherits: String,
    pub api_type: String,
    pub constants: Vec<Constant>,
    pub enums: Vec<Enum>,
    pub methods: Vec<Method>,
}

#[derive(Deserialize)]
pub struct Constructor {
    pub index: usize,
    pub arguments: Option<Vec<MethodArg>>,
}

#[derive(Deserialize)]
pub struct MethodArg {
    pub name: String,
    #[serde(rename = "type")]
    pub type_: String,
}

#[derive(Deserialize)]
pub struct MethodReturn {
    #[serde(rename = "type")]
    pub type_: String,
}

#[derive(Deserialize)]
pub struct Enum {
    pub name: String,
    pub values: Vec<Constant>,
}

#[derive(Deserialize)]
pub struct Constant {
    pub name: String,
    pub value: i32,
}

#[derive(Deserialize)]
pub struct Method {
    pub name: String,
    pub is_const: bool,
    pub is_vararg: bool,
    pub is_static: bool,
    pub is_virtual: bool,
    pub hash: u64,
    pub arguments: Vec<MethodArg>,
    pub return_value: Option<MethodReturn>,
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Implementation

pub fn load_extension_api() -> (ExtensionApi, &'static str) {
    // For float/double inference, see:
    // * https://github.com/godotengine/godot-proposals/issues/892
    // * https://github.com/godotengine/godot-cpp/pull/728
    let build_config = "float_64"; // TODO infer this

    let json: String = godot_exe::load_extension_api_json();
    let model: ExtensionApi = json::from_str(&json).expect("failed to deserialize JSON");
    (model, build_config)
}
