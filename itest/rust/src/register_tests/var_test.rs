/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::prelude::*;

use crate::framework::itest;

#[derive(GodotClass)]
#[class(init)]
struct WithInitDefaults {
    #[var(no_set)]
    default_int: i64,

    #[var(no_set)]
    #[init(val = 42)]
    literal_int: i64,

    #[var(no_set)]
    #[init(val = -42)]
    expr_int: i64,
}

// Test orthogonality of getter and setter.
#[derive(GodotClass)]
#[class(init, base=Node)]
struct VarAccessors {
    // Generated getter + setter.
    #[var]
    a_default: i32,

    // Custom getter (default name), generated setter. GString to also test a non-Copy type.
    #[var(get)]
    b_get: GString,

    // Generated getter, custom setter (default name).
    #[var(set)]
    c_set: i32,

    // Custom getter (custom name), generated setter.
    #[var(get = my_custom_get)]
    d_myget: i32,

    // Generated getter, custom setter (custom name).
    #[var(set = my_custom_set)]
    e_myset: i32,

    // Read-only (no setter).
    #[var(no_set)]
    f_noset: i32,

    // Write-only (no getter).
    #[var(no_get)]
    g_noget: i32,

    // Custom getter (custom name), no setter.
    #[var(get = my_custom_get, no_set)]
    h_myget_noset: i32,

    // No getter, custom setter (custom name).
    #[var(no_get, set = my_custom_set)]
    i_noget_myset: i32,

    // Custom getter (custom name), custom setter (custom name).
    #[var(get = my_custom_get, set = my_custom_set)]
    j_myget_myset: i32,
}

#[godot_api]
impl VarAccessors {
    // Custom getter for b).
    #[func]
    pub fn get_b_get(&self) -> GString {
        self.b_get.clone()
    }

    // Custom setter for c).
    #[func]
    pub fn set_c_set(&mut self, value: i32) {
        self.c_set = value;
    }

    // Custom getter, shared by d), h), j).
    #[func]
    pub fn my_custom_get(&self) -> i32 {
        self.d_myget + self.h_myget_noset + self.j_myget_myset
    }

    // Custom setter, shared by e), i), j).
    #[func]
    pub fn my_custom_set(&mut self, value: i32) {
        self.e_myset = value;
        self.i_noget_myset = value;
        self.j_myget_myset = value;
    }

    // Helper for GDScript to read write-only fields.
    #[func]
    fn gdscript_get(&self, field: GString) -> i32 {
        if field == "g" {
            self.g_noget
        } else if field == "i" {
            self.i_noget_myset
        } else {
            panic!("unknown field: {field}")
        }
    }
}

#[itest]
fn var_orthogonal_getters_setters() {
    let mut obj = VarAccessors::new_alloc();

    // a) generated getter + setter.
    obj.bind_mut().set_a_default(1);
    assert_eq!(obj.bind().get_a_default(), 1);
    assert!(obj.has_method("get_a_default"));
    assert!(obj.has_method("set_a_default"));

    // b) explicit getter (default name), generated setter. GString for type coverage.
    obj.bind_mut().set_b_get(GString::from("two"));
    assert_eq!(obj.bind().get_b_get(), "two");
    assert!(obj.has_method("get_b_get"));
    assert!(obj.has_method("set_b_get"));

    // c) generated getter, explicit setter (default name).
    obj.bind_mut().set_c_set(3);
    assert_eq!(obj.bind().get_c_set(), 3);
    assert!(obj.has_method("get_c_set"));
    assert!(obj.has_method("set_c_set"));

    // d) custom getter (custom name), generated setter.
    obj.bind_mut().set_d_myget(4);
    assert_eq!(obj.bind().my_custom_get(), 4); // d+h+j (h=j=0).
    assert!(obj.has_method("my_custom_get"));
    assert!(obj.has_method("set_d_myget"));

    // e) generated getter, custom setter (custom name).
    obj.bind_mut().my_custom_set(5); // Sets e, i, j to 5.
    assert_eq!(obj.bind().get_e_myset(), 5);
    assert!(obj.has_method("get_e_myset"));
    assert!(obj.has_method("my_custom_set"));

    // f) read-only (no setter).
    obj.bind_mut().f_noset = 6;
    assert_eq!(obj.bind().get_f_noset(), 6);
    assert!(obj.has_method("get_f_noset"));
    assert!(!obj.has_method("set_f_noset"));

    // g) write-only (no getter).
    obj.bind_mut().set_g_noget(7);
    assert_eq!(obj.bind().g_noget, 7);
    assert!(obj.has_method("set_g_noget"));
    assert!(!obj.has_method("get_g_noget"));

    // h) custom getter (custom name), no setter.
    obj.bind_mut().h_myget_noset = 8;
    assert_eq!(obj.bind().my_custom_get(), 4 + 8 + 5); // d+h+j.
    assert!(obj.has_method("my_custom_get"));
    assert!(!obj.has_method("set_h_myget_noset"));

    // i) no getter, custom setter (custom name).
    obj.bind_mut().my_custom_set(9); // Sets e, i, j to 9.
    assert_eq!(obj.bind().i_noget_myset, 9);
    assert!(obj.has_method("my_custom_set"));
    assert!(!obj.has_method("get_i_noget_myset"));

    // j) custom getter (custom name), custom setter (custom name).
    assert_eq!(obj.bind().j_myget_myset, 9); // Set by my_custom_set(9) above.
    assert_eq!(obj.bind().my_custom_get(), 4 + 8 + 9); // d+h+j.
    assert!(obj.has_method("my_custom_get"));
    assert!(obj.has_method("my_custom_set"));

    obj.free();
}
