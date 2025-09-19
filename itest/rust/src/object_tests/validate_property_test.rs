/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::builtin::{Array, Dictionary, GString, StringName};
use godot::classes::IObject;
use godot::global::{PropertyHint, PropertyUsageFlags};
use godot::meta::PropertyInfo;
use godot::obj::NewAlloc;
use godot::register::{godot_api, GodotClass};
use godot::test::itest;

#[derive(GodotClass)]
#[class(base = Object, init)]
pub struct ValidatePropertyTest {
    #[var(hint = NONE, hint_string = "initial")]
    #[export]
    my_var: i64,
}

#[godot_api]
impl IObject for ValidatePropertyTest {
    fn validate_property(&self, property: &mut PropertyInfo) {
        if property.property_name.to_string() == "my_var" {
            property.usage = PropertyUsageFlags::NO_EDITOR;
            property.property_name = StringName::from("SuperNewTestPropertyName");
            property.hint_info.hint_string = GString::from("SomePropertyHint");
            property.hint_info.hint = PropertyHint::TYPE_STRING;

            // Makes no sense, but allows to check if given ClassId can be properly moved to GDExtensionPropertyInfo.
            property.class_id = <ValidatePropertyTest as godot::obj::GodotClass>::class_id();
        }
    }
}

#[itest]
fn validate_property_test() {
    let obj = ValidatePropertyTest::new_alloc();
    let properties: Array<Dictionary> = obj.get_property_list();

    let property = properties
        .iter_shared()
        .find(|dict| {
            dict.get("name")
                .is_some_and(|v| v.to_string() == "SuperNewTestPropertyName")
        })
        .expect("Test failed – unable to find validated property.");

    let hint_string = property
        .get("hint_string")
        .expect("validated property dict should contain a `hint_string` entry.")
        .to::<GString>();
    assert_eq!(hint_string, GString::from("SomePropertyHint"));

    let class = property
        .get("class_name")
        .expect("Validated property dict should contain a class_name entry.")
        .to::<StringName>();
    assert_eq!(class, StringName::from("ValidatePropertyTest"));

    let usage = property
        .get("usage")
        .expect("Validated property dict should contain an usage entry.")
        .to::<PropertyUsageFlags>();
    assert_eq!(usage, PropertyUsageFlags::NO_EDITOR);

    let hint = property
        .get("hint")
        .expect("Validated property dict should contain a hint entry.")
        .to::<PropertyHint>();
    assert_eq!(hint, PropertyHint::TYPE_STRING);

    obj.free();
}
