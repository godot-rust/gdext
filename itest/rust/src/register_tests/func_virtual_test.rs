/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// Needed for Clippy to accept #[cfg(all())]
#![allow(clippy::non_minimal_cfg)]

use godot::builtin::vslice;
use godot::classes::GDScript;
use godot::global::godot_str;
use godot::prelude::*;

use crate::framework::{create_gdscript, itest};

#[derive(GodotClass)]
#[class(init)]
struct VirtualScriptCalls {
    _base: Base<RefCounted>,
}

#[godot_api]
impl VirtualScriptCalls {
    #[func(virtual)]
    fn greet_lang(&self, i: i32) -> GString {
        godot_str!("Rust#{i}")
    }

    #[func(virtual, rename = greet_lang2)]
    fn gl2(&self, s: GString) -> GString {
        godot_str!("{s} Rust")
    }

    #[func(virtual, gd_self)]
    fn greet_lang3(_this: Gd<Self>, s: GString) -> GString {
        godot_str!("{s} Rust")
    }

    #[func(virtual)]
    fn set_thing(&mut self, _input: Variant) {
        panic!("set_thing() must be overridden")
    }

    #[func(virtual)]
    fn get_thing(&self) -> Variant {
        panic!("get_thing() must be overridden")
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Tests

#[itest]
fn func_virtual() {
    // Without script: "Rust".
    let mut object = VirtualScriptCalls::new_gd();
    assert_eq!(object.bind().greet_lang(72), GString::from("Rust#72"));

    // With script: "GDScript".
    object.set_script(&make_script());
    assert_eq!(object.bind().greet_lang(72), GString::from("GDScript#72"));

    // Dynamic call: "GDScript".
    let result = object.call("_greet_lang", vslice![72]);
    assert_eq!(result, "GDScript#72".to_variant());
}

#[itest]
fn func_virtual_renamed() {
    // Without script: "Rust".
    let mut object = VirtualScriptCalls::new_gd();
    assert_eq!(
        object.bind().gl2("Hello".into()),
        GString::from("Hello Rust")
    );

    // With script: "GDScript".
    object.set_script(&make_script());
    assert_eq!(
        object.bind().gl2("Hello".into()),
        GString::from("Hello GDScript")
    );

    // Dynamic call: "GDScript".
    let result = object.call("greet_lang2", vslice!["Hello"]);
    assert_eq!(result, "Hello GDScript".to_variant());
}

#[itest]
fn func_virtual_gd_self() {
    // Without script: "Rust".
    let mut object = VirtualScriptCalls::new_gd();
    assert_eq!(
        VirtualScriptCalls::greet_lang3(object.clone(), "Hoi".into()),
        GString::from("Hoi Rust")
    );

    // With script: "GDScript".
    object.set_script(&make_script());
    assert_eq!(
        VirtualScriptCalls::greet_lang3(object.clone(), "Hoi".into()),
        GString::from("Hoi GDScript")
    );

    // Dynamic call: "GDScript".
    let result = object.call("_greet_lang3", vslice!["Hoi"]);
    assert_eq!(result, "Hoi GDScript".to_variant());
}

#[itest]
fn func_virtual_stateful() {
    let mut object = VirtualScriptCalls::new_gd();
    object.set_script(&make_script());

    let variant = Vector3i::new(1, 2, 2).to_variant();
    object.bind_mut().set_thing(variant.clone());

    let retrieved = object.bind().get_thing();
    assert_eq!(retrieved, variant);
}

fn make_script() -> Gd<GDScript> {
    let code = r#"
extends VirtualScriptCalls

var thing

func _greet_lang(i: int) -> String:
    return str("GDScript#", i)
    
func greet_lang2(s: String) -> String:
    return str(s, " GDScript")

func _greet_lang3(s: String) -> String:
    return str(s, " GDScript")

func _set_thing(anything):
    thing = anything

func _get_thing():
    return thing
"#;

    let mut script = create_gdscript(code);

    let methods = script
        .get_script_method_list()
        .iter_shared()
        .map(|dict| dict.get("name").unwrap())
        .collect::<VarArray>();

    // Ensure script has been parsed + compiled correctly.
    assert_eq!(script.get_instance_base_type(), "VirtualScriptCalls");
    assert_eq!(
        methods,
        varray![
            "_greet_lang",
            "greet_lang2",
            "_greet_lang3",
            "_set_thing",
            "_get_thing"
        ]
    );

    script
}
