/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::prelude::*;

pub(crate) fn run() -> bool {
    let ok = true;
    // No tests currently, tests using HasProperty are in Godot scripts.

    ok
}

#[derive(GodotClass)]
#[class(base=Node)]
struct HasProperty {
    #[base]
    base: Base<Node>,
    #[export(
        getter = "get_int_val",
        setter = "set_int_val",
        variant_type = "::godot::sys::VariantType::Int"
    )]
    int_val: i32,
    #[export(
        getter = "get_string_val",
        setter = "set_string_val",
        variant_type = "::godot::sys::VariantType::String"
    )]
    string_val: GodotString,
    #[export(
        getter = "get_object_val",
        setter = "set_object_val",
        variant_type = "::godot::sys::VariantType::Object"
    )]
    object_val: Option<Gd<Object>>,
}

#[godot_api]
impl HasProperty {
    #[func]
    pub fn get_int_val(&self) -> i32 {
        return self.int_val;
    }

    #[func]
    pub fn set_int_val(&mut self, val: i32) {
        self.int_val = val;
    }

    #[func]
    pub fn get_string_val(&self) -> GodotString {
        return self.string_val.clone();
    }

    #[func]
    pub fn set_string_val(&mut self, val: GodotString) {
        self.string_val = val;
    }

    #[func]
    pub fn get_object_val(&self) -> Variant {
        if let Some(object_val) = self.object_val.as_ref() {
            return object_val.to_variant();
        } else {
            return Variant::nil();
        }
    }

    #[func]
    pub fn set_object_val(&mut self, val: Gd<Object>) {
        self.object_val = Some(val);
    }
}

#[godot_api]
impl GodotExt for HasProperty {
    fn init(base: Base<Node>) -> Self {
        HasProperty {
            int_val: 0,
            object_val: None,
            string_val: GodotString::new(),
            base,
        }
    }
}
