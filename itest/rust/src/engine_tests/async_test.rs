/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::future::Future;
use std::ops::Deref;
use std::pin::pin;
use std::task::{Context, Poll, Waker};

use godot::builtin::{Array, Callable, Signal, array, iarray, vslice};
use godot::classes::{Object, RefCounted};
use godot::obj::{Base, Gd, NewAlloc, NewGd, WithBaseField};
use godot::prelude::{GodotClass, godot_api};
use godot::task::{self, SignalFuture, TaskHandle, create_test_signal_future_resolver};

use crate::framework::{TestContext, expect_async_panic, itest};

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

    // Mirrors the `spawn()` doc example. Verifies two things:
    // 1. The `base_mut()` guard allows the bind() during spawn()'s eager poll (up to the first `.await`).
    // 2. The async fn drops its bind() before awaiting, so no object reference is held across suspension.
    fn guarded_spawn(&mut self) -> TaskHandle {
        let this = self.to_gd();
        let _guard = self.base_mut();

        task::spawn(Self::wait_for_signal(this))
    }

    // Takes `Gd<Self>` by value, mirroring the recommended pattern -- never holds a bind across `.await`.
    async fn wait_for_signal(this: Gd<Self>) {
        let guard = this.bind(); // doesn't panic due to outer base_mut() guard.
        drop(guard); // before await point.

        this.signals().custom_signal().to_future().await;
    }
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
        vslice![iarray![1, 2, 3], ref_counted_arg],
    );

    task_handle
}

#[itest]
fn cancel_async_task(ctx: &TestContext) {
    let tree = ctx.scene_tree.get_tree();
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

// Regression test for https://github.com/godot-rust/gdext/issues/1624:
// A `SignalFuture` whose object is freed during engine teardown must not panic, but be silently cancelled.
#[itest]
fn signal_future_cancelled_at_engine_exit() {
    // We simulate teardown and free the object synchronously, so the whole flow is deterministic. The guard resets the flag on drop.
    let _exiting_guard = task::simulate_engine_exiting();

    let obj = Object::new_alloc();
    let signal = Signal::from_object_signal(&obj, "script_changed");

    let mut future = pin!(signal.to_future::<()>());
    let mut cx = Context::from_waker(Waker::noop());

    // First poll registers the one-shot connection and parks.
    assert_eq!(future.as_mut().poll(&mut cx), Poll::Pending);

    // Freeing the object drops the resolver:
    // * In regular use, this marks the future dead and the next poll would panic.
    // * While the engine is shutting down, the resolver leaves the future `Pending` instead, so it parks silently.
    obj.free();

    assert_eq!(
        future.as_mut().poll(&mut cx),
        Poll::Pending,
        "SignalFuture must park silently (not panic) when its object is freed during engine teardown"
    );
}

#[cfg(feature = "experimental-threads")]
#[itest(async)]
fn signal_future_non_send_arg_panic() -> TaskHandle {
    use godot::sys;

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

    // Important: this test deliberately frees the signal's object on another thread -- do NOT "fix" by keeping `object` alive here.
    // `object` is the only strong ref to the `RefCounted`. Moved into the worker thread (below), which emits the signal then drops the `Gd`
    // at its closure's end -- freeing the `RefCounted` off the main thread. So when the `SignalFuture` is dropped on the main thread, the
    // object is already gone, exercising the drop-after-free path guarded in `FallibleSignalFuture::drop` (formerly aborted the process).
    // Retaining a strong ref here would mask the bug. See https://github.com/godot-rust/gdext/pull/1617.
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
    let unrelated_callable = Callable::from_fn("unrelated", |_| {});

    assert_eq!(callable, cloned_callable);
    assert_ne!(callable, unrelated_callable);
    assert_ne!(cloned_callable, unrelated_callable);
}

// See guarded_spawn().
#[itest(async)]
fn async_task_bind_before_await() -> TaskHandle {
    let mut object = AsyncRefCounted::new_gd();
    let handle = object.bind_mut().guarded_spawn();
    object.signals().custom_signal().emit(0);

    handle
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
