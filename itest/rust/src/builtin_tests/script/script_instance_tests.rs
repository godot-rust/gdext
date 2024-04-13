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
            prop_list: vec![PropertyInfo {
                variant_type: VariantType::Int,
                property_name: StringName::from("script_property_a"),
                class_name: ClassName::from_ascii_cstr("\0".as_bytes()),
                hint: PropertyHint::NONE,
                hint_string: GString::new(),
                usage: PropertyUsageFlags::NONE,
            }],

            method_list: vec![MethodInfo {
                id: 1,
                method_name: StringName::from("script_method_a"),
                class_name: ClassName::from_ascii_cstr("TestScript\0".as_bytes()),
                return_type: PropertyInfo {
                    variant_type: VariantType::String,
                    class_name: ClassName::none(),
                    property_name: StringName::from(""),
                    hint: PropertyHint::NONE,
                    hint_string: GString::new(),
                    usage: PropertyUsageFlags::NONE,
                },
                arguments: vec![
                    PropertyInfo {
                        variant_type: VariantType::String,
                        class_name: ClassName::none(),
                        property_name: StringName::from(""),
                        hint: PropertyHint::NONE,
                        hint_string: GString::new(),
                        usage: PropertyUsageFlags::NONE,
                    },
                    PropertyInfo {
                        variant_type: VariantType::Int,
                        class_name: ClassName::none(),
                        property_name: StringName::from(""),
                        hint: PropertyHint::NONE,
                        hint_string: GString::new(),
                        usage: PropertyUsageFlags::NONE,
                    },
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

    fn set_property(&mut self, name: impl Into<StringName>, value: &Variant) -> bool {
        if name.into().to_string() == "script_property_b" {
            self.script_property_b = FromGodot::from_variant(value);
            true
        } else {
            false
        }
    }

    fn get_property(&self, name: impl Into<StringName>) -> Option<Variant> {
        match name.into().to_string().as_str() {
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
        &mut self,
        method: impl Into<StringName>,
        args: &[&Variant],
    ) -> Result<Variant, sys::GDExtensionCallErrorType> {
        match method.into().to_string().as_str() {
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

    fn has_method(&self, method: impl Into<StringName>) -> bool {
        matches!(method.into().to_string().as_str(), "script_method_a")
    }

    fn get_script(&self) -> &Gd<Script> {
        &self.script
    }

    fn get_property_type(&self, name: impl Into<StringName>) -> VariantType {
        match name.into().to_string().as_str() {
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
