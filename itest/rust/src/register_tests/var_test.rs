/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
use godot::global::godot_str;
use godot::prelude::*;

use crate::framework::{expect_panic, itest};

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
    #[var( get = my_custom_get)]
    d_myget: i32,

    // Generated getter, custom setter (custom name).
    #[var( set = my_custom_set)]
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

    // Public generated getter + setter.
    #[var(pub)]
    k_pub: i32,

    // Custom getter, public generated setter.
    #[var(pub, get = my_l_get)]
    l_pub_get: GString,

    // A field named `instance` caused compilation error in versions before 0.5.
    // See: https://github.com/godot-rust/gdext/pull/1481.
    #[var]
    instance: i32,
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

    // Custom getter for l).
    #[func]
    pub fn my_l_get(&self) -> GString {
        godot_str!(".:{}:.", self.l_pub_get)
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
    obj.set("a_default", &1.to_variant());
    assert_eq!(obj.get("a_default"), 1.to_variant());
    assert!(obj.has_method("get_a_default"));
    assert!(obj.has_method("set_a_default"));

    // b) explicit getter (default name), generated setter. GString for type coverage.
    obj.bind_mut().b_get = GString::from("two");
    assert_eq!(obj.bind().get_b_get(), "two");
    assert!(obj.has_method("get_b_get"));
    assert!(obj.has_method("set_b_get"));

    // c) generated getter, explicit setter (default name).
    obj.bind_mut().set_c_set(3);
    assert_eq!(obj.bind().c_set, 3);
    assert!(obj.has_method("get_c_set"));
    assert!(obj.has_method("set_c_set"));

    // d) custom getter (custom name), generated setter.
    obj.bind_mut().d_myget = 4;
    assert_eq!(obj.bind().my_custom_get(), 4); // d+h+j (h=j=0).
    assert!(obj.has_method("my_custom_get"));
    assert!(obj.has_method("set_d_myget"));

    // e) generated getter, custom setter (custom name).
    obj.bind_mut().my_custom_set(5); // Sets e, i, j to 5.
    assert_eq!(obj.bind().e_myset, 5);
    assert!(obj.has_method("get_e_myset"));
    assert!(obj.has_method("my_custom_set"));

    // f) read-only (no setter).
    obj.bind_mut().f_noset = 6;
    assert_eq!(obj.get("f_noset"), 6.to_variant()); // Dynamic (no Rust getter).
    assert!(obj.has_method("get_f_noset"));
    assert!(!obj.has_method("set_f_noset"));

    // g) write-only (no getter).
    obj.set("g_noget", &7.to_variant()); // Dynamic (no Rust setter).
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

#[itest]
fn var_pub_accessors() {
    let mut obj = VarAccessors::new_alloc();

    // k) #[var(pub)] - both generated, no deprecation warning.
    obj.bind_mut().set_k_pub(10);
    assert_eq!(obj.bind().get_k_pub(), 10);
    assert!(obj.has_method("get_k_pub"));
    assert!(obj.has_method("set_k_pub"));

    // l) #[var(pub, get)] - custom getter visible, generated setter visible.
    obj.bind_mut().h_myget_noset = 3;
    obj.bind_mut().set_l_pub_get(GString::from("test")); // d+h+j (h=3).
    assert_eq!(obj.get("l_pub_get"), ".:test:.".to_variant());
    assert!(obj.has_method("my_l_get"));
    assert!(obj.has_method("set_l_pub_get"));

    obj.free();
}

#[itest]
fn var_deprecated_accessors() {
    let mut obj = VarAccessors::new_alloc();

    // a) #[var] - still generates old setters for backwards compatibility.
    #[expect(deprecated)]
    obj.bind_mut().set_a_default(5);
    #[expect(deprecated)]
    let a = obj.bind().get_a_default();
    assert_eq!(a, 5);

    // c) #[var(set)].
    #[expect(deprecated)]
    let c = obj.bind_mut().get_c_set();
    assert_eq!(c, 0);

    obj.free();
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Edge cases for #[var(pub)]

#[derive(GodotClass)]
#[class(base=Node)]
struct VarPubEdgeCases {
    base: Base<Node>,

    #[var(pub)]
    on_ready_int: OnReady<i32>,

    #[var(pub)]
    on_ready_node: OnReady<Gd<Node>>,

    #[var(pub)]
    on_editor_int: OnEditor<i32>,

    #[var(pub)]
    on_editor_node: OnEditor<Gd<Node>>,
}

#[godot_api]
impl INode for VarPubEdgeCases {
    fn init(base: Base<Node>) -> Self {
        Self {
            base,
            on_ready_int: OnReady::manual(),
            on_ready_node: OnReady::manual(),
            on_editor_int: OnEditor::from_sentinel(-1),
            on_editor_node: OnEditor::default(), // Option<Gd>::None.
        }
    }
}

#[itest]
fn var_pub_access_on_ready_builtin() {
    let mut obj = VarPubEdgeCases::new_alloc();
    obj.bind_mut().on_ready_int.init(42);

    assert_eq!(obj.bind().get_on_ready_int(), 42);

    obj.bind_mut().set_on_ready_int(100);
    assert_eq!(*obj.bind().on_ready_int, 100);

    obj.free();
}

#[itest]
fn var_pub_access_on_ready_gd() {
    let mut obj = VarPubEdgeCases::new_alloc();
    let first = Node::new_alloc();
    let second = Node::new_alloc();

    obj.bind_mut().on_ready_node.init(first.clone());
    assert_eq!(obj.bind().get_on_ready_node(), first);
    assert_eq!(&*obj.bind().on_ready_node, &first);

    obj.bind_mut().set_on_ready_node(second.clone());
    assert_eq!(obj.bind().get_on_ready_node(), second);

    first.free();
    second.free();
    obj.free();
}

#[itest]
fn var_pub_access_on_ready_panics() {
    let mut obj = VarPubEdgeCases::new_alloc();
    let node = Node::new_alloc();

    expect_panic("get - OnReady<Gd<Node>>", || {
        obj.bind().get_on_ready_node();
    });
    expect_panic("get - OnReady<i32>", || {
        obj.bind().get_on_ready_int();
    });
    expect_panic("set - OnReady<Gd<Node>>", || {
        obj.bind_mut().set_on_ready_node(node.clone());
    });
    expect_panic("set - OnReady<i32>", || {
        obj.bind_mut().set_on_ready_int(42);
    });

    node.free();
    obj.free();
}

#[itest]
fn var_pub_access_on_editor_builtin() {
    let mut obj = VarPubEdgeCases::new_alloc();

    assert_eq!(obj.bind().get_on_editor_int(), -1);

    obj.bind_mut().set_on_editor_int(42);
    assert_eq!(obj.bind().get_on_editor_int(), 42);
    assert_eq!(*obj.bind().on_editor_int, 42);

    obj.free();
}

#[itest]
fn var_pub_access_on_editor_gd() {
    let mut obj = VarPubEdgeCases::new_alloc();
    let node = Node::new_alloc();

    expect_panic("get - OnEditor<Gd<Node>>", || {
        obj.bind().get_on_editor_node();
    });

    obj.bind_mut().set_on_editor_node(node.clone());
    assert_eq!(obj.bind().get_on_editor_node(), node.clone());
    assert_eq!(&*obj.bind().on_editor_node, &node);

    node.free();
    obj.free();
}
