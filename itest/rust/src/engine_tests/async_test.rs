/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::ops::Deref;

use godot::builtin::{array, vslice, Array, Callable, Signal, Variant};
use godot::classes::{Object, RefCounted};
use godot::obj::{Base, Gd, NewAlloc, NewGd};
use godot::prelude::{godot_api, GodotClass};
use godot::sys;
use godot::task::{self, create_test_signal_future_resolver, SignalFuture, TaskHandle};

use crate::framework::{expect_async_panic, itest, TestContext};

#[derive(GodotClass)]
#[class(init)]
struct AsyncRefCounted {
    base: Base<RefCounted>,
}

#[godot_api]
impl AsyncRefCounted {
    #[signal]
    fn custom_signal(value: u32);
    #[signal]
    fn custom_signal_array(value: Array<i64>);
}

#[itest(async)]
fn start_async_task() -> TaskHandle {
    let mut object = RefCounted::new_gd();
    let object_ref = object.clone();
    let signal = Signal::from_object_signal(&object, "custom_signal");

    object.add_user_signal("custom_signal");

    let task_handle = task::spawn(async move {
        let signal_future: SignalFuture<(u8, Gd<RefCounted>)> = signal.to_future();
        let (result, object) = signal_future.await;

        assert_eq!(result, 10);
        assert!(object.is_instance_valid());

        drop(object_ref);
    });

    let ref_counted_arg = RefCounted::new_gd();

    object.emit_signal("custom_signal", vslice![10, ref_counted_arg]);

    task_handle
}

#[itest(async)]
fn async_task_array() -> TaskHandle {
    let mut object = RefCounted::new_gd();
    let signal = Signal::from_object_signal(&object, "custom_signal_array");

    object.add_user_signal("custom_signal_array");

    let task_handle = task::spawn(async move {
        let signal_future: SignalFuture<(Array<i64>, Gd<RefCounted>)> = signal.to_future();
        let (result, object) = signal_future.await;

        assert_eq!(result, array![1, 2, 3]);
        assert!(object.is_instance_valid());
    });

    let ref_counted_arg = RefCounted::new_gd();

    object.emit_signal(
        "custom_signal_array",
        vslice![array![1, 2, 3], ref_counted_arg],
    );

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

#[itest(async)]
fn async_task_signal_future_panic() -> TaskHandle {
    let mut obj = Object::new_alloc();

    let signal = Signal::from_object_signal(&obj, "script_changed");

    let handle = task::spawn(expect_async_panic(
        "future should panic when the signal object is dropped",
        async move {
            signal.to_future::<()>().await;
        },
    ));

    obj.call_deferred("free", &[]);

    handle
}

#[cfg(feature = "experimental-threads")]
#[itest(async)]
fn signal_future_non_send_arg_panic() -> TaskHandle {
    use crate::framework::ThreadCrosser;

    let mut object = RefCounted::new_gd();
    let signal = Signal::from_object_signal(&object, "custom_signal");

    object.add_user_signal("custom_signal");

    let handle = task::spawn(expect_async_panic(
        "future should panic when the Gd<RefCounted> is sent between threads",
        async move {
            signal.to_future::<(Gd<RefCounted>,)>().await;
        },
    ));

    // This test verifies that panics work if something is non-sendable. Since we can no longer safely invoke Drop in such a case,
    // the object (here RefCounted) is leaked. However, we don't want memory leaks in tests, so we do it differently:
    // Leaking a RefCounted can be counteracted by creating another RefCounted *weakly* (so it doesn't increase the refcount).
    // This is done at the end -- after moving the object out of the thread via escape pod.
    static ESCAPE_POD: sys::Global<Option<ThreadCrosser<Gd<RefCounted>>>> = sys::Global::default();

    let object = ThreadCrosser::new(object);

    let thread = std::thread::spawn(move || {
        let mut object = unsafe { object.extract() };

        let arg = RefCounted::new_gd();
        // Eject the RefCounted before panic explodes the thread.
        *ESCAPE_POD.lock() = Some(ThreadCrosser::new(arg.clone()));

        // This will panic:
        object.emit_signal("custom_signal", vslice![arg])
    });

    // Wait until thread concludes, also to avoid race conditions.
    thread.join().expect("failed to join thread");

    let escape_pod = ESCAPE_POD.lock().take().unwrap();
    let object = unsafe { escape_pod.extract() };
    let balance_restorer: Gd<RefCounted> = unsafe { Gd::__from_obj_sys_weak(object.obj_sys()) };
    drop(balance_restorer);

    handle
}

#[cfg(feature = "experimental-threads")]
#[itest(async)]
fn signal_future_send_arg_no_panic() -> TaskHandle {
    use crate::framework::ThreadCrosser;

    let mut object = RefCounted::new_gd();
    let signal = Signal::from_object_signal(&object, "custom_signal");

    object.add_user_signal("custom_signal");

    let handle = task::spawn(async move {
        let (value,) = signal.to_future::<(u8,)>().await;

        assert_eq!(value, 1);
    });

    let object = ThreadCrosser::new(object);

    std::thread::spawn(move || {
        let mut object = unsafe { object.extract() };

        object.emit_signal("custom_signal", vslice![1u8])
    });

    handle
}

// Test that two callables created from the same future resolver (but cloned) are equal, while they are not equal to an unrelated
// callable.
#[itest]
fn resolver_callabable_equality() {
    let resolver = create_test_signal_future_resolver::<(u8,)>();

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
    let copy = object.clone();

    let task_handle = task::spawn(async move {
        // Could also use to_future() instead of deref().
        let (result,) = copy.signals().custom_signal().deref().await;

        assert_eq!(result, 66);
    });

    object.signals().custom_signal().emit(66);

    task_handle
}

#[itest(async)]
fn async_typed_signal_with_array() -> TaskHandle {
    let object = AsyncRefCounted::new_gd();
    let copy = object.clone();

    let task_handle = task::spawn(async move {
        let (result,) = copy.signals().custom_signal_array().to_future().await;

        assert_eq!(result, array![1, 2, 3]);
    });

    object
        .signals()
        .custom_signal_array()
        .emit(&array![1, 2, 3]);

    task_handle
}
