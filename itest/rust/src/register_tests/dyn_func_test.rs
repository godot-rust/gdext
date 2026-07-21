/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Tests for `#[func]` inside `#[godot_dyn]` trait impls: Rust-side polymorphism through `DynGd`, while the same methods are callable
//! from Godot on the concrete class.

use godot::global::godot_str;
use godot::prelude::*;

use crate::framework::itest;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Example symbols

trait Rollbackable: 'static {
    /// Registered under its own name.
    fn rollback_tick(&mut self, tick: i64, is_fresh: bool) -> GString;

    /// Registered under a different Godot name.
    fn tick_count(&self) -> i64;

    /// Not registered with Godot at all -- Rust-only trait method.
    fn label(&self) -> GString;
}

#[derive(GodotClass)]
#[class(init)]
struct RollbackRefc {
    ticks: i64,
}

// A primary #[godot_api] impl block is required for the trait methods to be registered.
#[godot_api]
impl RollbackRefc {
    #[func]
    fn inherent_func(&self) -> i64 {
        -1
    }
}

#[godot_dyn]
impl Rollbackable for RollbackRefc {
    #[func]
    fn rollback_tick(&mut self, tick: i64, is_fresh: bool) -> GString {
        self.ticks += 1;
        godot_str!("RollbackRefc#{tick}/{is_fresh}")
    }

    #[func(rename = get_tick_count)]
    fn tick_count(&self) -> i64 {
        self.ticks
    }

    fn label(&self) -> GString {
        "refc".into()
    }
}

#[derive(GodotClass)]
#[class(init, base = Node)]
struct RollbackNode {
    ticks: i64,
    base: Base<Node>,
}

#[godot_api]
impl RollbackNode {}

#[godot_dyn]
impl Rollbackable for RollbackNode {
    #[func]
    fn rollback_tick(&mut self, tick: i64, is_fresh: bool) -> GString {
        self.ticks += 1;
        godot_str!("RollbackNode#{tick}/{is_fresh}")
    }

    #[func(rename = get_tick_count)]
    fn tick_count(&self) -> i64 {
        self.ticks
    }

    fn label(&self) -> GString {
        GString::from(&self.base().get_name())
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Tests

#[itest]
fn dyn_func_static_call() {
    let mut obj = RollbackRefc::new_gd();

    assert_eq!(
        obj.bind_mut().rollback_tick(7, true),
        GString::from("RollbackRefc#7/true")
    );
    assert_eq!(obj.bind().tick_count(), 1);
    assert_eq!(obj.bind().label(), GString::from("refc"));
}

#[itest]
fn dyn_func_registered_in_godot() {
    let mut obj = RollbackRefc::new_gd();

    assert!(obj.has_method("rollback_tick"));
    assert!(obj.has_method("get_tick_count"));
    assert!(obj.has_method("inherent_func"));

    // Not registered: neither under its Rust name (renamed) nor as a non-#[func] trait method.
    assert!(!obj.has_method("tick_count"));
    assert!(!obj.has_method("label"));

    let result = obj.call("rollback_tick", vslice![7, true]);
    assert_eq!(result, "RollbackRefc#7/true".to_variant());

    assert_eq!(obj.call("get_tick_count", &[]), 1.to_variant());
    assert_eq!(obj.call("inherent_func", &[]), (-1).to_variant());
}

#[itest]
fn dyn_func_polymorphic_call() {
    // The scenario from the issue: a collection of heterogeneous objects, called polymorphically through the trait.
    let refc = RollbackRefc::new_gd().into_dyn::<dyn Rollbackable>();
    let node = RollbackNode::new_alloc().into_dyn::<dyn Rollbackable>();

    let mut objects: Array<DynGd<Object, dyn Rollbackable>> = array![
        &refc.clone().upcast::<Object>(),
        &node.clone().upcast::<Object>()
    ];

    let mut results = vec![];
    for mut obj in objects.iter_shared() {
        results.push(obj.dyn_bind_mut().rollback_tick(3, false));
    }

    assert_eq!(
        results,
        vec![
            GString::from("RollbackRefc#3/false"),
            GString::from("RollbackNode#3/false")
        ]
    );

    // Same methods are reachable from Godot, on the concrete classes.
    for mut obj in objects.iter_shared() {
        assert_eq!(obj.call("get_tick_count", &[]), 1.to_variant());
    }

    objects.clear();
    node.free();
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Virtual dispatch to GDScript overrides

#[cfg(since_api = "4.3")]
mod virtual_dyn {
    use godot::classes::GDScript;
    use godot::global::godot_str;
    use godot::prelude::*;

    use crate::framework::{create_gdscript, itest};

    pub trait Speaker: 'static {
        fn speak(&self, i: i32) -> GString;
    }

    #[derive(GodotClass)]
    #[class(init)]
    pub struct DynSpeaker {
        _base: Base<RefCounted>,
    }

    #[godot_api]
    impl DynSpeaker {}

    #[godot_dyn]
    impl Speaker for DynSpeaker {
        // Registered under the plain name `speak`; a script override wins over this default, from both Godot and Rust.
        #[func(virtual_pub)]
        fn speak(&self, i: i32) -> GString {
            godot_str!("Rust#{i}")
        }
    }

    fn make_script() -> Gd<GDScript> {
        create_gdscript(
            r#"
extends DynSpeaker

@warning_ignore("native_method_override")
func speak(i: int) -> String:
    return str("GDScript#", i)
"#,
        )
    }

    #[itest]
    fn dyn_func_virtual_pub() {
        let mut object = DynSpeaker::new_gd();

        // Without script: Rust default, from Godot and Rust.
        assert_eq!(object.call("speak", vslice![72]), "Rust#72".to_variant());
        assert_eq!(object.bind().speak(72), GString::from("Rust#72"));

        // Also through the trait object.
        let dyn_obj = object.clone().into_dyn::<dyn Speaker>();
        assert_eq!(dyn_obj.dyn_bind().speak(72), GString::from("Rust#72"));

        // With script: the override wins in all three cases.
        object.set_script(&make_script());
        assert_eq!(
            object.call("speak", vslice![72]),
            "GDScript#72".to_variant()
        );
        assert_eq!(object.bind().speak(72), GString::from("GDScript#72"));
        assert_eq!(dyn_obj.dyn_bind().speak(72), GString::from("GDScript#72"));
    }
}
