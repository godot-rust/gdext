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
    stored_value: i64,
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

    #[func]
    fn store(&mut self, value: i64) {
        self.stored_value = value;
    }

    #[func]
    fn store_via_self_call(&mut self) {
        self.first_called_pre = true;

        // The argument expression reads `self.first_called_pre`; self_call! hoists it to a local before
        // releasing `self` for the call() -- the pattern that a direct `self.base_mut().call(...)` can't
        // express, since the argument would still be borrowing `self` while `base_mut()` also does.
        self_call!(self.call("store", &[(self.first_called_pre as i64).to_variant()]));

        self.first_called_post = true;
    }
}

#[itest]
fn reentrant_call_succeeds() {
    let mut class = ReentrantClass::new_alloc();

    assert!(!class.bind().first_called_pre);
    assert!(!class.bind().first_called_post);
    assert!(!class.bind().second_called);

    class.call("first_calls", &[]);

    assert!(class.bind().first_called_pre);
    assert!(class.bind().first_called_post);
    assert!(class.bind().second_called);

    class.free()
}

#[itest]
fn reentrant_emit_succeeds() {
    let mut class = ReentrantClass::new_alloc();

    let callable = class.callable("second");
    class.connect("some_signal", &callable);

    assert!(!class.bind().first_called_pre);
    assert!(!class.bind().first_called_post);
    assert!(!class.bind().second_called);

    class.call("first_signal", &[]);

    assert!(class.bind().first_called_pre);
    assert!(class.bind().first_called_post);
    assert!(class.bind().second_called);

    class.free()
}

#[itest]
fn reentrant_closure_call_succeeds() {
    let mut class = ReentrantClass::new_alloc();

    assert!(!class.bind().first_called_pre);
    assert!(!class.bind().first_called_post);
    assert!(!class.bind().second_called);

    class.call("first_calls_reentrant", &[]);

    assert!(class.bind().first_called_pre);
    assert!(class.bind().first_called_post);
    assert!(class.bind().second_called);

    class.free()
}

#[itest]
fn self_call_macro_hoists_args() {
    let mut class = ReentrantClass::new_alloc();

    assert_eq!(class.bind().stored_value, 0);

    class.call("store_via_self_call", &[]);

    // `first_called_pre` was `true` (1) at the point the macro's argument expression was evaluated.
    assert_eq!(class.bind().stored_value, 1);
    assert!(class.bind().first_called_post);

    class.free()
}
