/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::prelude::*;

use crate::framework::itest;

#[derive(GodotClass)]
#[class(init, base = Object)]
pub struct ReentrantClass {
    base: Base<Object>,

    first_called_pre: bool,
    first_called_post: bool,
    second_called: bool,
}

#[godot_api]
impl ReentrantClass {
    #[signal(__no_builder)]
    fn some_signal();

    #[func]
    fn first_calls(&mut self) {
        self.first_called_pre = true;
        self.base_mut().call("second", &[]);
        self.first_called_post = true;
    }

    #[func]
    fn first_signal(&mut self) {
        self.first_called_pre = true;
        self.base_mut().emit_signal("some_signal", &[]);
        self.first_called_post = true;
    }

    #[func]
    fn first_calls_reentrant(&mut self) {
        self.first_called_pre = true;
        self.reentrant(|base| {
            base.call("second", &[]);
        });
        self.first_called_post = true;
    }

    #[func]
    fn second(&mut self) {
        self.second_called = true;
    }
}

fn check_reentrant(method: &str, setup: impl FnOnce(&mut Gd<ReentrantClass>)) {
    let mut class = ReentrantClass::new_alloc();
    setup(&mut class);

    assert!(!class.bind().first_called_pre);
    assert!(!class.bind().first_called_post);
    assert!(!class.bind().second_called);

    class.call(method, &[]);

    assert!(class.bind().first_called_pre);
    assert!(class.bind().first_called_post);
    assert!(class.bind().second_called);

    class.free()
}

#[itest]
fn reentrant_call_succeeds() {
    check_reentrant("first_calls", |_| {});
}

#[itest]
fn reentrant_emit_succeeds() {
    check_reentrant("first_signal", |class| {
        let callable = class.callable("second");
        class.connect("some_signal", &callable);
    });
}

#[itest]
fn reentrant_closure_call_succeeds() {
    check_reentrant("first_calls_reentrant", |_| {});
}
