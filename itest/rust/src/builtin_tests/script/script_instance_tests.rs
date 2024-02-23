/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::ffi::c_void;

use godot::builtin::meta::{ClassName, FromGodot, MethodInfo, PropertyInfo, ToGodot};
use godot::builtin::{GString, StringName, Variant, VariantType};
use godot::engine::global::{MethodFlags, PropertyHint, PropertyUsageFlags};
use godot::engine::{
    create_script_instance, IScriptExtension, Object, Script, ScriptExtension, ScriptInstance,
    ScriptLanguage,
};
use godot::obj::{Base, Gd, WithBaseField};
use godot::register::{godot_api, GodotClass};
use godot::sys;

#[derive(GodotClass)]
#[class(base = ScriptExtension, init)]
struct TestScript {
    base: Base<ScriptExtension>,
}

#[godot_api]
impl IScriptExtension for TestScript {
    unsafe fn instance_create(&self, _for_object: Gd<Object>) -> *mut c_void {
        create_script_instance(TestScriptInstance::new(self.to_gd().upcast()))
    }

    fn can_instantiate(&self) -> bool {
        true
    }
}

struct TestScriptInstance {
    /// A field to store the value of the `script_property_b` during tests.
    script_property_b: bool,
    prop_list: Vec<PropertyInfo>,
    method_list: Vec<MethodInfo>,
    script: Gd<Script>,
}

impl TestScriptInstance {
    fn new(script: Gd<Script>) -> Self {
        Self {
            script,
            script_property_b: false,
            prop_list: vec![PropertyInfo::new_var::<i64>("script_property_a")],

            method_list: vec![MethodInfo {
                id: 1,
                method_name: StringName::from("script_method_a"),
                class_name: ClassName::from_ascii_cstr("TestScript\0".as_bytes()),
                return_type: PropertyInfo::new_return::<GString>(),
                arguments: vec![
                    PropertyInfo::new_arg::<GString>(""),
                    PropertyInfo::new_arg::<i64>(""),
                ],
                default_arguments: vec![],
                flags: MethodFlags::NORMAL,
            }],
        }
    }
}

impl ScriptInstance for TestScriptInstance {
    fn class_name(&self) -> GString {
        GString::from("TestScript")
    }

    fn set(&mut self, name: StringName, value: &Variant) -> bool {
        if name.to_string() == "script_property_b" {
            self.script_property_b = FromGodot::from_variant(value);
            true
        } else {
            false
        }
    }

    fn get(&self, name: StringName) -> Option<Variant> {
        match name.to_string().as_str() {
            "script_property_a" => Some(Variant::from(10)),
            "script_property_b" => Some(Variant::from(self.script_property_b)),
            _ => None,
        }
    }

    fn get_property_list(&self) -> &[PropertyInfo] {
        &self.prop_list
    }

    fn get_method_list(&self) -> &[MethodInfo] {
        &self.method_list
    }

    fn call(
        &mut self,
        method: StringName,
        args: &[&Variant],
    ) -> Result<Variant, sys::GDExtensionCallErrorType> {
        match method.to_string().as_str() {
            "script_method_a" => {
                let arg_a = args[0].to::<GString>();
                let arg_b = args[1].to::<i32>();

                Ok(format!("{arg_a}{arg_b}").to_variant())
            }

            _ => Err(sys::GDEXTENSION_CALL_ERROR_INVALID_METHOD),
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
            "script_property_a" => VariantType::Int,
            _ => VariantType::Nil,
        }
    }

    fn to_string(&self) -> GString {
        GString::from("script instance to string")
    }

    fn get_property_state(&self) -> Vec<(StringName, Variant)> {
        panic!("property_state is not implemented")
    }

    fn get_language(&self) -> Gd<ScriptLanguage> {
        panic!("language is not implemented")
    }

    fn on_refcount_decremented(&self) -> bool {
        true
    }

    fn on_refcount_incremented(&self) {}

    fn property_get_fallback(&self, _name: StringName) -> Option<Variant> {
        None
    }

    fn property_set_fallback(&mut self, _name: StringName, _value: &Variant) -> bool {
        false
    }
}
