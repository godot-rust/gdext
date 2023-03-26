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

    #[export]
    int_val: i32,
    #[export(get = get_int_val_read)]
    int_val_read: i32,
    #[export(set = set_int_val_write)]
    int_val_write: i32,
    #[export(get = get_int_val_rw, set = set_int_val_rw)]
    int_val_rw: i32,
    #[export(get = get_int_val_getter, set)]
    int_val_getter: i32,
    #[export(get, set = set_int_val_setter)]
    int_val_setter: i32,

    #[export(get = get_string_val, set = set_string_val)]
    string_val: GodotString,
    #[export(get = get_object_val, set = set_object_val)]
    object_val: Option<Gd<Object>>,
    #[export]
    texture_val: Gd<Texture>,
    #[export(get = get_texture_val, set = set_texture_val, hint = PROPERTY_HINT_RESOURCE_TYPE, hint_desc = "Texture")]
    texture_val_rw: Option<Gd<Texture>>,
}

#[godot_api]
impl HasProperty {
    #[func]
    pub fn get_int_val_read(&self) -> i32 {
        self.int_val_read
    }

    #[func]
    pub fn set_int_val_write(&mut self, val: i32) {
        self.int_val_write = val;
    }

    // Odd name to make sure it doesn't interfere with "get_*".
    #[func]
    pub fn retrieve_int_val_write(&mut self) -> i32 {
        self.int_val_write
    }

    #[func]
    pub fn get_int_val_rw(&self) -> i32 {
        self.int_val_rw
    }

    #[func]
    pub fn set_int_val_rw(&mut self, val: i32) {
        self.int_val_rw = val;
    }

    #[func]
    pub fn get_int_val_getter(&self) -> i32 {
        self.int_val_getter
    }

    #[func]
    pub fn set_int_val_setter(&mut self, val: i32) {
        self.int_val_setter = val;
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
    pub fn get_texture_val_rw(&self) -> Variant {
        if let Some(texture_val) = self.texture_val_rw.as_ref() {
            texture_val.to_variant()
        } else {
            Variant::nil()
        }
    }

    #[func]
    pub fn set_texture_val_rw(&mut self, val: Gd<Texture>) {
        self.texture_val_rw = Some(val);
    }
}

#[godot_api]
impl NodeVirtual for HasProperty {
    fn init(base: Base<Node>) -> Self {
        HasProperty {
            int_val: 0,
            int_val_read: 2,
            int_val_write: 0,
            int_val_rw: 0,
            int_val_getter: 0,
            int_val_setter: 0,
            object_val: None,
            string_val: GodotString::new(),
            texture_val: Texture::new(),
            texture_val_rw: None,
            base,
        }
    }
}
