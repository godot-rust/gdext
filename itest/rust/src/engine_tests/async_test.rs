/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::builtin::godot_task;
use godot::builtin::Signal;
use godot::classes::{Engine, Object, SceneTree};
use godot::global::godot_print;
use godot::obj::NewAlloc;

use crate::framework::itest;

async fn call_async_fn(signal: Signal) -> u8 {
    let value = 5;

    let _: () = signal.to_future().await;

    value + 5
}

#[itest]
fn start_async_task() {
    let tree = Engine::singleton()
        .get_main_loop()
        .unwrap()
        .cast::<SceneTree>();

    let signal = Signal::from_object_signal(&tree, "process_frame");

    godot_print!("starting godot_task...");
    godot_task(async move {
        godot_print!("running async task...");

        godot_print!("starting nested task...");

        let inner_signal = signal.clone();
        godot_task(async move {
            godot_print!("inside nested task...");

            let _: () = inner_signal.to_future().await;

            godot_print!("nested task after await...");
            godot_print!("nested task done!");
        });

        let result = call_async_fn(signal.clone()).await;
        godot_print!("got async result...");

        assert_eq!(result, 10);
        godot_print!("assertion done, async task complete!");
    });
    godot_print!("after godot_task...");
}

#[itest]
fn cancel_async_task() {
    let tree = Engine::singleton()
        .get_main_loop()
        .unwrap()
        .cast::<SceneTree>();

    let signal = Signal::from_object_signal(&tree, "process_frame");

    let handle = godot_task(async move {
        godot_print!("starting task to be canceled...");

        let _: () = signal.to_future().await;

        unreachable!();
    });

    handle.cancel();
}

#[itest]
fn async_task_guaranteed_signal_future() {
    let mut obj = Object::new_alloc();

    let signal = Signal::from_object_signal(&obj, "script_changed");

    godot_task(async move {
        godot_print!("starting task with guaranteed signal future...");

        let result: Option<()> = signal.to_guaranteed_future().await;

        assert!(result.is_none());

        godot_print!("task asserted!");
    });

    obj.call_deferred("free", &[]);
}
