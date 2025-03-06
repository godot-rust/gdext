/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::ops::Deref;

use godot::builtin::{Callable, Signal, Variant};
use godot::classes::{Object, RefCounted};
use godot::meta::ToGodot;
use godot::obj::{Base, Gd, NewAlloc, NewGd};
use godot::prelude::{godot_api, GodotClass};
use godot::task::{self, SignalFuture, SignalFutureResolver, TaskHandle};

use crate::framework::{itest, TestContext};

#[derive(GodotClass)]
#[class(init)]
struct AsyncRefCounted {
    base: Base<RefCounted>,
}

#[godot_api]
impl AsyncRefCounted {
    #[signal]
    fn custom_signal(value: u32);
}

#[itest(async)]
fn start_async_task() -> TaskHandle {
    let mut object = RefCounted::new_gd();
    let object_ref = object.clone();
    let signal = Signal::from_object_signal(&object, "custom_signal");

    object.add_user_signal("custom_signal");

    let task_handle = task::spawn(async move {
        let signal_future: SignalFuture<(u8,)> = signal.to_future();
        let (result,) = signal_future.await;

        assert_eq!(result, 10);
        drop(object_ref);
    });

    object.emit_signal("custom_signal", &[10.to_variant()]);

    task_handle
}

#[itest]
fn cancel_async_task(ctx: &TestContext) {
    let tree = ctx.scene_tree.get_tree().unwrap();
    let signal = Signal::from_object_signal(&tree, "process_frame");

    let handle = task::spawn(async move {
        let _: () = signal.to_future().await;

        unreachable!();
    });

    handle.cancel();
}

#[itest(async)]
fn async_task_fallible_signal_future() -> TaskHandle {
    let mut obj = Object::new_alloc();

    let signal = Signal::from_object_signal(&obj, "script_changed");

    let handle = task::spawn(async move {
        let result = signal.to_fallible_future::<()>().await;

        assert!(result.is_err());
    });

    obj.call_deferred("free", &[]);

    handle
}

// Test that two callables created from the same future resolver (but cloned) are equal, while they are not equal to an unrelated
// callable.
#[itest]
fn resolver_callabable_equality() {
    let resolver = SignalFutureResolver::<(u8,)>::default();

    let callable = Callable::from_custom(resolver.clone());
    let cloned_callable = Callable::from_custom(resolver.clone());
    let unrelated_callable = Callable::from_local_fn("unrelated", |_| Ok(Variant::nil()));

    assert_eq!(callable, cloned_callable);
    assert_ne!(callable, unrelated_callable);
    assert_ne!(cloned_callable, unrelated_callable);
}

#[itest(async)]
fn async_typed_signal() -> TaskHandle {
    let object = AsyncRefCounted::new_gd();
    let object_ref = object.clone();

    let task_handle = task::spawn(async move {
        let (result,) = object.signals().custom_signal().deref().await;

        assert_eq!(result, 66);
    });

    object_ref.signals().custom_signal().emit(66);

    task_handle
}
