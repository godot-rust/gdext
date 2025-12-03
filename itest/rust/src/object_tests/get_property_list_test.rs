/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::collections::HashMap;

use godot::builtin::{GString, StringName, VarDictionary, VariantType, Vector2, Vector3};
use godot::classes::{IObject, Node};
use godot::global::{PropertyHint, PropertyUsageFlags};
use godot::meta::PropertyInfo;
use godot::obj::{Gd, NewAlloc};
use godot::register::{godot_api, GodotClass};
use godot::test::itest;

#[derive(GodotClass)]
#[class(base = Object, init)]
pub struct GetPropertyListTest {}

#[godot_api]
impl IObject for GetPropertyListTest {
    fn get_property_list(&mut self) -> Vec<PropertyInfo> {
        vec![
            PropertyInfo::new_var::<bool>("my_property"),
            PropertyInfo::new_export::<GString>("a_string_property"),
            PropertyInfo::new_group("some_group", "some_group_"),
            PropertyInfo::new_export::<Vector2>("some_group_my_vector_2"),
            PropertyInfo::new_export::<Vector3>("some_group_my_vector_3"),
            PropertyInfo::new_subgroup("my_subgroup", "some_subgroup_"),
            PropertyInfo::new_export::<Option<Gd<Node>>>("some_subgroup_node"),
        ]
    }
}

fn property_dict_eq_property_info(dict: &VarDictionary, info: &PropertyInfo) -> bool {
    dict.get("name").unwrap().to::<GString>().to_string() == info.property_name.to_string()
        && dict.get("class_name").unwrap().to::<StringName>() == info.class_id.to_string_name()
        && dict.get("type").unwrap().to::<VariantType>() == info.variant_type
        && dict.get("hint").unwrap().to::<PropertyHint>() == info.hint_info.hint
        && dict.get("hint_string").unwrap().to::<GString>() == info.hint_info.hint_string
        && dict.get("usage").unwrap().to::<PropertyUsageFlags>() == info.usage
}

#[itest]
fn get_property_list_returns() {
    let mut obj = GetPropertyListTest::new_alloc();

    let properties = obj.get_property_list();

    let mut properties_missing = HashMap::new();

    properties_missing.extend(
        obj.bind_mut()
            .get_property_list()
            .into_iter()
            .map(|prop| (prop.property_name.to_string(), prop)),
    );

    for dict in properties.iter_shared() {
        let name = dict.get("name").unwrap().to::<GString>();

        let Some(prop) = properties_missing.get(&name.to_string()) else {
            continue;
        };

        if property_dict_eq_property_info(&dict, prop) {
            properties_missing.remove(&name.to_string());
        }
    }

    assert!(
        properties_missing.is_empty(),
        "missing properties: {:?}",
        properties_missing.keys().collect::<Vec<_>>()
    );

    obj.free();
}
