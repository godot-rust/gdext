/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::ffi::c_void;

use godot::builtin::{Array, Dictionary, GString, StringName, Variant, VariantType};
use godot::classes::{
    IScriptExtension, IScriptLanguageExtension, Object, Script, ScriptExtension, ScriptLanguage,
    ScriptLanguageExtension,
};
use godot::global::{Error, MethodFlags};
use godot::meta::{ClassName, FromGodot, MethodInfo, PropertyInfo, ToGodot};
use godot::obj::script::{create_script_instance, ScriptInstance, SiMut};
use godot::obj::{Base, Gd, NewAlloc, WithBaseField};
use godot::register::{godot_api, GodotClass};
use godot::sys;

use crate::framework::itest;

#[derive(GodotClass)]
#[class(base = ScriptExtension, no_init, tool)]
struct TestScript {
    language: Gd<TestScriptLanguage>,
    base: Base<ScriptExtension>,
}

impl TestScript {
    fn new(language: Gd<TestScriptLanguage>) -> Gd<Self> {
        Gd::from_init_fn(|base| Self { language, base })
    }
}

#[rustfmt::skip]
#[godot_api]
impl IScriptExtension for TestScript {
    fn can_instantiate(&self) -> bool {
        true
    }

    unsafe fn instance_create_rawptr(&self, for_object: Gd<Object>) -> *mut c_void {
        create_script_instance(TestScriptInstance::new(self.to_gd().upcast()), for_object)
    }

    fn get_language(&self) -> Option<Gd<ScriptLanguage>> {
        Some(self.language.clone().upcast())
    }

    fn editor_can_reload_from_file(&mut self) -> bool { unreachable!() }
    fn get_base_script(&self) -> Option<Gd<Script>> { unreachable!() }
    fn get_global_name(&self) -> StringName { unreachable!() }
    fn inherits_script(&self, _script: Gd<Script>) -> bool { unreachable!() }
    fn get_instance_base_type(&self) -> StringName { unreachable!() }
    unsafe fn placeholder_instance_create_rawptr(&self, _for_object: Gd<Object>) -> *mut c_void { unreachable!() }
    fn instance_has(&self, _object: Gd<Object>) -> bool { unreachable!() }
    fn has_source_code(&self) -> bool { unreachable!() }
    fn get_source_code(&self) -> GString { unreachable!() }
    fn set_source_code(&mut self, _code: GString) { unreachable!() }
    fn reload(&mut self, _keep_state: bool) -> Error { unreachable!() }
    fn get_documentation(&self) -> Array<Dictionary> { unreachable!() }
    fn has_method(&self, _method: StringName) -> bool { unreachable!() }
    fn has_static_method(&self, _method: StringName) -> bool { unreachable!() }
    fn get_method_info(&self, _method: StringName) -> Dictionary { unreachable!() }
    fn is_tool(&self) -> bool { unreachable!() }
    fn is_valid(&self) -> bool { unreachable!() }
    fn has_script_signal(&self, _signall: StringName) -> bool { unreachable!() }
    fn get_script_signal_list(&self) -> Array<Dictionary> { unreachable!() }
    fn has_property_default_value(&self, _property: StringName) -> bool { unreachable!() }
    fn get_property_default_value(&self, _property: StringName) -> Variant { unreachable!() }
    fn update_exports(&mut self) { unreachable!() }
    fn get_script_method_list(&self) -> Array<Dictionary> { unreachable!() }
    fn get_script_property_list(&self) -> Array<Dictionary> { unreachable!() }
    fn get_member_line(&self, _member: StringName) -> i32 { unreachable!() }
    fn get_constants(&self) -> Dictionary { unreachable!() }
    fn get_members(&self) -> Array<StringName> { unreachable!() }
    fn is_placeholder_fallback_enabled(&self) -> bool { unreachable!() }
    fn get_rpc_config(&self) -> Variant { unreachable!() }
    
    #[cfg(since_api = "4.4")]
    fn get_doc_class_name(&self) -> StringName { unreachable!() }
}

struct TestScriptInstance {
    /// A field to store the value of the `script_property_b` during tests.
    script_property_b: bool,
    prop_list: Vec<PropertyInfo>,
    method_list: Vec<MethodInfo>,
    script: Gd<Script>,
    script_language: Gd<ScriptLanguage>,
}

impl TestScriptInstance {
    fn new(script: Gd<TestScript>) -> Self {
        Self {
            script_language: {
                let s = script.bind();

                s.get_language().unwrap()
            },
            script: script.upcast(),
            script_property_b: false,
            prop_list: vec![PropertyInfo::new_var::<i64>("script_property_a")],

            method_list: vec![MethodInfo {
                id: 1,
                method_name: StringName::from("script_method_a"),
                class_name: ClassName::new_cached::<TestScript>(|| "TestScript".to_string()),
                return_type: PropertyInfo::new_var::<GString>(""),
                arguments: vec![
                    PropertyInfo::new_var::<GString>("arg_a"),
                    PropertyInfo::new_var::<i32>("arg_b"),
                ],
                default_arguments: vec![],
                flags: MethodFlags::NORMAL,
            }],
        }
    }

    /// Method of the test script and will be called during test runs.
    fn script_method_a(&self, arg_a: GString, arg_b: i32) -> String {
        format!("{arg_a}{arg_b}")
    }

    fn script_method_toggle_property_b(&mut self) -> bool {
        self.script_property_b = !self.script_property_b;
        true
    }
}

impl ScriptInstance for TestScriptInstance {
    type Base = Object;

    fn class_name(&self) -> GString {
        GString::from("TestScript")
    }

    fn set_property(mut this: SiMut<Self>, name: StringName, value: &Variant) -> bool {
        if name.to_string() == "script_property_b" {
            this.script_property_b = FromGodot::from_variant(value);
            true
        } else {
            false
        }
    }

    fn get_property(&self, name: StringName) -> Option<Variant> {
        match name.to_string().as_str() {
            "script_property_a" => Some(Variant::from(10)),
            "script_property_b" => Some(Variant::from(self.script_property_b)),
            _ => None,
        }
    }

    fn get_property_list(&self) -> Vec<PropertyInfo> {
        self.prop_list.clone()
    }

    fn get_method_list(&self) -> Vec<MethodInfo> {
        self.method_list.clone()
    }

    fn call(
        mut this: SiMut<Self>,
        method: StringName,
        args: &[&Variant],
    ) -> Result<Variant, sys::GDExtensionCallErrorType> {
        match method.to_string().as_str() {
            "script_method_a" => {
                let arg_a = args[0].to::<GString>();
                let arg_b = args[1].to::<i32>();

                Ok(this.script_method_a(arg_a, arg_b).to_variant())
            }

            "script_method_toggle_property_b" => {
                let result = this.script_method_toggle_property_b();

                Ok(result.to_variant())
            }

            "script_method_re_entering" => {
                let mut base = this.base_mut();
                let result = base.call("script_method_toggle_property_b", &[]);

                Ok(result)
            }

            other => {
                println!("CALL: {other} with args: {args:?}");
                Err(sys::GDEXTENSION_CALL_ERROR_INVALID_METHOD)
            }
        }
    }

    fn is_placeholder(&self) -> bool {
        panic!("is_placeholder is not implemented")
    }

    fn has_method(&self, method: StringName) -> bool {
        matches!(method.to_string().as_str(), "script_method_a")
    }

    fn get_script(&self) -> &Gd<Script> {
        &self.script
    }

    fn get_property_type(&self, name: StringName) -> VariantType {
        match name.to_string().as_str() {
            "script_property_a" => VariantType::INT,
            _ => VariantType::NIL,
        }
    }

    fn to_string(&self) -> GString {
        GString::from("script instance to string")
    }

    fn get_property_state(&self) -> Vec<(StringName, Variant)> {
        panic!("property_state is not implemented")
    }

    fn get_language(&self) -> Gd<ScriptLanguage> {
        self.script_language.clone()
    }

    fn on_refcount_decremented(&self) -> bool {
        true
    }

    fn on_refcount_incremented(&self) {}

    fn property_get_fallback(&self, _name: StringName) -> Option<Variant> {
        None
    }

    fn property_set_fallback(_this: SiMut<Self>, _name: StringName, _value: &Variant) -> bool {
        false
    }

    #[cfg(since_api = "4.3")]
    fn get_method_argument_count(&self, _method: StringName) -> Option<u32> {
        None
    }
}

#[derive(GodotClass)]
#[class(base = ScriptLanguageExtension, tool, init)]
struct TestScriptLanguage {
    base: Base<ScriptLanguageExtension>,
}

#[godot_api]
impl TestScriptLanguage {
    // In a real implementation the script would be created by the engine via ScriptLanguageExtension::create_script(), via a
    // ResourceFormatLoader or by creating a new instance of TestScript. Most likely the script language would be registered as an engine
    // singleton to make it globally available.
    #[func]
    fn new_script(&self) -> Gd<TestScript> {
        TestScript::new(self.to_gd().cast())
    }
}

#[godot_api]
#[rustfmt::skip]
impl IScriptLanguageExtension for TestScriptLanguage {
    fn get_name(&self) -> GString { unreachable!() }
    fn init_ext(&mut self) { unreachable!() }
    fn get_type(&self) -> GString { unreachable!() }
    fn get_extension(&self) -> GString { unreachable!() }
    fn finish(&mut self) { unreachable!() }
    fn get_reserved_words(&self) -> godot::prelude::PackedStringArray { unreachable!() }
    fn is_control_flow_keyword(&self, _keyword: GString) -> bool { unreachable!() }
    fn get_comment_delimiters(&self) -> godot::prelude::PackedStringArray { unreachable!() }
    fn get_string_delimiters(&self) -> godot::prelude::PackedStringArray { unreachable!() }
    fn make_template(&self, _template: GString, _class_name: GString, _base_class_name: GString) -> Option<Gd<Script>> { unreachable!() }
    fn get_built_in_templates(&self, _object: StringName) -> Array<Dictionary> { unreachable!() }
    fn is_using_templates(&mut self) -> bool { unreachable!() }
    fn validate(&self, _script: GString, _path: GString, _validate_functions: bool, _validate_errors: bool, _validate_warnings: bool, _validate_safe_lines: bool) -> Dictionary { unreachable!() }
    fn validate_path(&self, _path: GString) -> GString { unreachable!() }
    fn create_script(&self) -> Option<Gd<Object>> { unreachable!() }
    fn has_named_classes(&self) -> bool { unreachable!() }
    fn supports_builtin_mode(&self) -> bool { unreachable!() }
    fn supports_documentation(&self) -> bool { unreachable!() }
    fn can_inherit_from_file(&self) -> bool { unreachable!() }
    fn find_function(&self, _class_name: GString, _function_namee: GString) -> i32 { unreachable!() }
    fn make_function(&self, _class_name: GString, _function_name: GString, _function_args: godot::prelude::PackedStringArray) -> GString { unreachable!() }
    fn open_in_external_editor(&mut self, _script: Option<Gd<Script>>, _line: i32, _column: i32) -> godot::global::Error { unreachable!() }
    fn overrides_external_editor(&mut self) -> bool { unreachable!() }
    fn complete_code(&self, _code: GString,_pathh: GString, _ownerer: Option<Gd<Object>>) -> Dictionary { unreachable!() }
    fn lookup_code(&self, _code: GString, _symbol: GString, _path: GString, _owner: Option<Gd<Object>>) -> Dictionary { unreachable!() }
    fn auto_indent_code(&self, _code: GString, _from_linee: i32, _to_line: i32) -> GString { unreachable!() }
    fn add_global_constant(&mut self, _name: StringName,_valuee: Variant) { unreachable!() }
    fn add_named_global_constant(&mut self, _name: StringName,_valuee: Variant) { unreachable!() }
    fn remove_named_global_constant(&mut self, _name: StringName) { unreachable!() }
    fn thread_enter(&mut self) { unreachable!() }
    fn thread_exit(&mut self) { unreachable!() }
    fn debug_get_error(&self) -> GString { unreachable!() }
    fn debug_get_stack_level_count(&self) -> i32 { unreachable!() }
    fn debug_get_stack_level_line(&self, _level: i32) -> i32 { unreachable!() }
    fn debug_get_stack_level_function(&self, _level: i32) -> GString { unreachable!() }
    fn debug_get_stack_level_locals(&mut self, _level: i32, _max_subitems: i32, _max_depth: i32) -> Dictionary { unreachable!() }
    fn debug_get_stack_level_members(&mut self, _level: i32, _max_subitems: i32, _max_depth: i32) -> Dictionary { unreachable!() }
    unsafe fn debug_get_stack_level_instance_rawptr(&mut self, _level: i32) -> *mut c_void { unreachable!() }
    fn debug_get_globals(&mut self, _max_subitems: i32,_max_depthh: i32) -> Dictionary { unreachable!() }
    fn debug_parse_stack_level_expression(&mut self, _level: i32, _expression: GString, _max_subitems: i32, _max_depth: i32) -> GString { unreachable!() }
    fn debug_get_current_stack_info(&mut self) -> Array<Dictionary> { unreachable!() }
    fn reload_all_scripts(&mut self) { unreachable!() }
    fn reload_tool_script(&mut self, _script: Option<Gd<Script>>,_soft_reloadd: bool) { unreachable!() }
    fn get_recognized_extensions(&self) -> godot::prelude::PackedStringArray { unreachable!() }
    fn get_public_functions(&self) -> Array<Dictionary> { unreachable!() }
    fn get_public_constants(&self) -> Dictionary { unreachable!() }
    fn get_public_annotations(&self) -> Array<Dictionary> { unreachable!() }
    fn profiling_start(&mut self) { unreachable!() }
    fn profiling_stop(&mut self) { unreachable!() }
    unsafe fn profiling_get_accumulated_data_rawptr(&mut self, _info_array: *mut godot::classes::native::ScriptLanguageExtensionProfilingInfo, _info_max: i32) -> i32 { unreachable!() }
    unsafe fn profiling_get_frame_data_rawptr(&mut self, _info_array: *mut godot::classes::native::ScriptLanguageExtensionProfilingInfo, _info_max: i32) -> i32 { unreachable!() }
    fn frame(&mut self) { unreachable!() }
    fn handles_global_class_type(&self, _type_: GString) -> bool { unreachable!() }
    fn get_global_class_name(&self, _path: GString) -> Dictionary { unreachable!() }
    #[cfg(since_api = "4.3")]
    fn profiling_set_save_native_calls(&mut self, _enable: bool) { unreachable!() }
    #[cfg(since_api = "4.3")]
    fn debug_get_stack_level_source(&self, _level: i32) -> GString { unreachable!() }
    #[cfg(since_api = "4.3")]
    fn can_make_function(&self) -> bool { unreachable!() }
    #[cfg(since_api = "4.3")]
    fn preferred_file_name_casing(&self) -> godot::classes::script_language::ScriptNameCasing { unreachable!() }
    #[cfg(since_api = "4.4")]
    fn reload_scripts(&mut self, _scripts: Array<Variant>, _soft: bool) { unreachable!() }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Test Cases

// Test that [`script_instance_exists`] returns true if a instance of a script exists for the given object.
#[itest]
fn script_instance_exists() {
    let language = TestScriptLanguage::new_alloc();
    let script = TestScript::new(language.clone());
    let mut object = Object::new_alloc();

    object.set_script(&script.to_variant());

    let instance_exists = godot::obj::script::script_instance_exists(&object, &script);
    assert!(instance_exists);

    object.set_script(&Variant::nil());

    let instance_exists = godot::obj::script::script_instance_exists(&object, &script);
    assert!(!instance_exists);

    object.free();
    language.free();
}
