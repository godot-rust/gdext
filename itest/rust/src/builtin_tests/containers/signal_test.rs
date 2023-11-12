/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::cell::Cell;

use godot::bind::{godot_api, GodotClass};
use godot::builtin::{GString, Variant};

use godot::engine::Object;
use godot::obj::{Base, Gd, UserClass};
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
    #[base]
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
        assert_eq!(self.base.clone(), arg1);
        assert_eq!(SIGNAL_ARG_STRING, arg2.to_string());

        self.used[2].set(true);
    }
}

const SIGNAL_ARG_STRING: &str = "Signal string arg";

#[itest]
/// Test that godot can call a method that is connect with a signal
fn signals() {
    let mut emitter = Emitter::alloc_gd();
    let receiver = Receiver::alloc_gd();

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
