/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::{engine::Texture, prelude::*};

// No tests currently, tests using HasProperty are in Godot scripts.

#[derive(GodotClass)]
#[class(base=Node)]
struct HasProperty {
    #[base]
    base: Base<Node>,
    #[export(getter = "get_int_val", setter = "set_int_val")]
    int_val: i32,
    #[export(getter = "get_string_val", setter = "set_string_val")]
    string_val: GodotString,
    #[export(getter = "get_object_val", setter = "set_object_val")]
    object_val: Option<Gd<Object>>,
    #[export(getter = "get_texture_val", setter = "set_texture_val", hint = PROPERTY_HINT_RESOURCE_TYPE, hint_desc = "Texture")]
    texture_val: Option<Gd<Texture>>,
}

#[godot_api]
impl HasProperty {
    #[func]
    pub fn get_int_val(&self) -> i32 {
        self.int_val
    }

    #[func]
    pub fn set_int_val(&mut self, val: i32) {
        self.int_val = val;
    }

    #[func]
    pub fn get_string_val(&self) -> GodotString {
        self.string_val.clone()
    }

    #[func]
    pub fn set_string_val(&mut self, val: GodotString) {
        self.string_val = val;
    }

    #[func]
    pub fn get_object_val(&self) -> Variant {
        if let Some(object_val) = self.object_val.as_ref() {
            object_val.to_variant()
        } else {
            Variant::nil()
        }
    }

    #[func]
    pub fn set_object_val(&mut self, val: Gd<Object>) {
        self.object_val = Some(val);
    }

    #[func]
    pub fn get_texture_val(&self) -> Variant {
        if let Some(texture_val) = self.texture_val.as_ref() {
            texture_val.to_variant()
        } else {
            Variant::nil()
        }
    }

    #[func]
    pub fn set_texture_val(&mut self, val: Gd<Texture>) {
        self.texture_val = Some(val);
    }
}

#[godot_api]
impl GodotExt for HasProperty {
    fn init(base: Base<Node>) -> Self {
        HasProperty {
            int_val: 0,
            object_val: None,
            string_val: GodotString::new(),
            texture_val: None,
            base,
        }
    }
}
