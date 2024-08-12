/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::ffi::c_void;

use godot::builtin::{Array, Dictionary, GString, StringName, Variant, VariantType};
use godot::classes::{IScriptExtension, Object, Script, ScriptExtension, ScriptLanguage};
use godot::global::{Error, MethodFlags};
use godot::meta::{ClassName, FromGodot, MethodInfo, PropertyInfo, ToGodot};
use godot::obj::script::{create_script_instance, ScriptInstance, SiMut};
use godot::obj::{Base, Gd, WithBaseField};
use godot::register::{godot_api, GodotClass};
use godot::sys;

#[derive(GodotClass)]
#[class(base = ScriptExtension, init, tool)]
struct TestScript {
    base: Base<ScriptExtension>,
}

#[rustfmt::skip]
#[godot_api]
impl IScriptExtension for TestScript {
    fn can_instantiate(&self) -> bool {
        true
    }

    unsafe fn instance_create(&self, for_object: Gd<Object>) -> *mut c_void {
        create_script_instance(TestScriptInstance::new(self.to_gd().upcast()), for_object)
    }

    fn editor_can_reload_from_file(&mut self) -> bool { unreachable!() }
    fn get_base_script(&self) -> Option<Gd<Script>> { unreachable!() }
    fn get_global_name(&self) -> StringName { unreachable!() }
    fn inherits_script(&self, _script: Gd<Script>) -> bool { unreachable!() }
    fn get_instance_base_type(&self) -> StringName { unreachable!() }
    unsafe fn placeholder_instance_create(&self, _for_object: Gd<Object>) -> *mut c_void { unreachable!() }
    fn instance_has(&self, _object: Gd<Object>) -> bool { unreachable!() }
    fn has_source_code(&self) -> bool { unreachable!() }
    fn get_source_code(&self) -> GString { unreachable!() }
    fn set_source_code(&mut self, _code: GString) { unreachable!() }
    fn reload(&mut self, _keep_state: bool) -> Error { unreachable!() }
    fn get_documentation(&self) -> Array<Dictionary> { unreachable!() }
    fn has_method(&self, _method: StringName) -> bool { unreachable!() }
    #[cfg(since_api = "4.2")]
    fn has_static_method(&self, _method: StringName) -> bool { unreachable!() }
    fn get_method_info(&self, _method: StringName) -> Dictionary { unreachable!() }
    fn is_tool(&self) -> bool { unreachable!() }
    fn is_valid(&self) -> bool { unreachable!() }
    fn get_language(&self) -> Option<Gd<ScriptLanguage>> { unreachable!() }
    fn has_script_signal(&self, _signall: StringName) -> bool { unreachable!() }
    fn get_script_signal_list(&self) -> Array<Dictionary> { unreachable!() }
    fn has_property_default_value(&self, _property: StringName) -> bool { unreachable!() }
    fn get_property_default_value(&self, _property: StringName) -> Variant { unreachable!() }
    fn update_exports(&mut self) { unreachable!() }
    fn get_script_method_list(&self) -> Array<Dictionary> { unreachable!() }
    fn get_script_property_list(&self) -> Array<Dictionary> { unreachable!() }
    fn get_member_line(&self, _member: StringName) -> i32 { unreachable!() }
    fn get_constants(&self) -> godot::prelude::Dictionary { unreachable!() }
    fn get_members(&self) -> godot::prelude::Array<StringName> { unreachable!() }
    fn is_placeholder_fallback_enabled(&self) -> bool { unreachable!() }
    fn get_rpc_config(&self) -> Variant { unreachable!() }
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
                let result = base.call("script_method_toggle_property_b".into(), &[]);

                Ok(result)
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
        panic!("language is not implemented")
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
