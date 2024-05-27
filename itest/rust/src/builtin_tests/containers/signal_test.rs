/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::cell::Cell;

use godot::builtin::{Callable, GString, Signal, StringName, Variant};
use godot::meta::ToGodot;
use godot::register::{godot_api, GodotClass};

use godot::classes::{Object, RefCounted};
use godot::obj::{Base, Gd, NewAlloc, NewGd, WithBaseField};
use godot::sys;

use crate::framework::itest;

#[derive(GodotClass)]
#[class(init, base=Object)]
struct Emitter {}

#[godot_api]
impl Emitter {
    #[signal]
    fn signal_0_arg();

    #[signal]
    fn signal_1_arg(arg1: i64);

    #[signal]
    fn signal_2_arg(arg1: Gd<Object>, arg2: GString);
}

#[derive(GodotClass)]
#[class(init, base=Object)]
struct Receiver {
    used: [Cell<bool>; 3],
    base: Base<Object>,
}

#[godot_api]
impl Receiver {
    #[func]
    fn receive_0_arg(&self) {
        self.used[0].set(true);
    }

    #[func]
    fn receive_1_arg(&self, arg1: i64) {
        self.used[1].set(true);
        assert_eq!(arg1, 987);
    }

    #[func]
    fn receive_2_arg(&self, arg1: Gd<Object>, arg2: GString) {
        assert_eq!(self.base().clone(), arg1);
        assert_eq!(SIGNAL_ARG_STRING, arg2.to_string());

        self.used[2].set(true);
    }
}

const SIGNAL_ARG_STRING: &str = "Signal string arg";

#[itest]
/// Test that godot can call a method that is connect with a signal
fn signals() {
    let mut emitter = Emitter::new_alloc();
    let receiver = Receiver::new_alloc();

    let args = [
        vec![],
        vec![Variant::from(987)],
        vec![
            Variant::from(receiver.clone()),
            Variant::from(SIGNAL_ARG_STRING),
        ],
    ];

    for (i, arg) in args.iter().enumerate() {
        let signal_name = format!("signal_{i}_arg");
        let receiver_name = format!("receive_{i}_arg");

        emitter.connect(signal_name.clone().into(), receiver.callable(receiver_name));
        emitter.emit_signal(signal_name.into(), arg);

        assert!(receiver.bind().used[i].get());
    }

    receiver.free();
    emitter.free();
}

#[itest]
fn instantiate_signal() {
    let mut object = RefCounted::new_gd();

    object.add_user_signal("test_signal".into());

    let signal = Signal::from_object_signal(&object, "test_signal");

    assert!(!signal.is_null());
    assert_eq!(signal.name(), StringName::from("test_signal"));
    assert_eq!(signal.object().unwrap(), object.clone().upcast());
    assert_eq!(signal.object_id().unwrap(), object.instance_id());
}

#[itest]
fn emit_signal() {
    let mut object = RefCounted::new_gd();

    object.add_user_signal("test_signal".into());

    let signal = Signal::from_object_signal(&object, "test_signal");
    let receiver = Receiver::new_alloc();

    object.connect(
        StringName::from("test_signal"),
        Callable::from_object_method(&receiver, "receive_1_arg"),
    );

    assert_eq!(signal.connections().len(), 1);

    signal.emit(&[987i64.to_variant()]);

    assert!(receiver.bind().used[1].get());

    receiver.free();
}

#[itest]
fn connect_signal() {
    let mut object = RefCounted::new_gd();

    object.add_user_signal("test_signal".into());

    let signal = Signal::from_object_signal(&object, "test_signal");
    let receiver = Receiver::new_alloc();

    signal.connect(Callable::from_object_method(&receiver, "receive_1_arg"), 0);

    assert_eq!(signal.connections().len(), 1);

    object.emit_signal(StringName::from("test_signal"), &[987i64.to_variant()]);

    assert!(receiver.bind().used[1].get());

    receiver.free();
}
