/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// Testing dynamic properties exposed mainly through `get_property_list` and other helper methods.

use crate::framework::itest;

use godot::builtin::meta::{PropertyInfo, ToGodot};
use godot::builtin::{GString, StringName, VariantArray};
use godot::engine::{INode, Node, Object};
use godot::obj::{Gd, Inherits, NewAlloc};
use godot::register::{godot_api, GodotClass};

#[derive(GodotClass)]
#[class(init, base = Node)]
struct PropertyListTest {
    toggle_props: bool,
}

#[godot_api]
impl INode for PropertyListTest {
    fn get_property_list(&self) -> Vec<PropertyInfo> {
        let mut properties = vec![
            PropertyInfo::new_var::<i64>("some_i64_property"),
            PropertyInfo::new_var::<GString>("some_gstring_property"),
            PropertyInfo::new_var::<VariantArray>("some_variantarray_property"),
        ];

        if self.toggle_props {
            properties.push(PropertyInfo::new_var::<Option<Gd<Node>>>(
                "some_toggled_property",
            ));
        }

        properties
    }
}

fn has_property<T: Inherits<Object>, S: Into<StringName>>(gd: &Gd<T>, property: S) -> bool {
    let gd = gd.clone().upcast::<Object>();
    let property = property.into();
    let property_list = gd.get_property_list();

    for prop in property_list.iter_shared() {
        if prop.get("name") == Some(property.to_variant()) {
            return true;
        }
    }

    false
}

#[itest]
fn property_list_has_property() {
    let mut property_list_test = PropertyListTest::new_alloc();

    assert!(has_property(&property_list_test, "some_i64_property"));
    assert!(has_property(&property_list_test, "some_gstring_property"));
    assert!(has_property(
        &property_list_test,
        "some_variantarray_property"
    ));
    assert!(!has_property(&property_list_test, "some_toggled_property"));
    assert!(!has_property(
        &property_list_test,
        "some_undefined_property"
    ));

    property_list_test.bind_mut().toggle_props = true;

    assert!(has_property(&property_list_test, "some_i64_property"));
    assert!(has_property(&property_list_test, "some_gstring_property"));
    assert!(has_property(
        &property_list_test,
        "some_variantarray_property"
    ));
    assert!(has_property(&property_list_test, "some_toggled_property"));
    assert!(!has_property(
        &property_list_test,
        "some_undefined_property"
    ));

    property_list_test.free();
}
