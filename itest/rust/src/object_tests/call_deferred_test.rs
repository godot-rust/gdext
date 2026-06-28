/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::cell::Cell;
use std::ops::DerefMut;
use std::rc::Rc;

use godot::obj::WithBaseField;
use godot::prelude::*;
use godot::task::{self, SignalFuture, TaskHandle};

use crate::framework::itest;

const ACCEPTED_NAME: &str = "touched";

trait ForwardTrait {
    fn forward(&mut self);
}

#[derive(GodotClass)]
#[class(init,base=Node2D)]
struct DeferredTestNode {
    base: Base<Node2D>,
}

#[godot_api]
impl DeferredTestNode {
    #[signal]
    fn test_completed(name: StringName);

    #[func]
    fn accept(&mut self) {
        self.base_mut().set_name(ACCEPTED_NAME);
    }

    fn accept_gd(mut this: Gd<Self>) {
        this.set_name(ACCEPTED_NAME);
    }

    fn accept_dyn_gd(mut this: DynGd<Object, dyn ForwardTrait>) {
        this.dyn_bind_mut().forward();
    }

    fn create_assertion_task(&mut self) -> TaskHandle {
        assert_ne!(
            self.base().get_name().to_string(),
            ACCEPTED_NAME,
            "accept evaluated synchronously"
        );

        let run_test: SignalFuture<(StringName,)> = self.signals().test_completed().to_future();

        godot::task::spawn(async move {
            let (name,) = run_test.await;

            assert_eq!(name.to_string(), ACCEPTED_NAME);
        })
    }
}

#[godot_api]
impl INode2D for DeferredTestNode {
    fn process(&mut self, _delta: f64) {
        let name = self.base().get_name();
        self.signals().test_completed().emit(&name);
        self.base_mut().queue_free();
    }

    fn ready(&mut self) {
        self.base_mut().set_name("verify");
    }
}

#[godot_dyn]
impl ForwardTrait for DeferredTestNode {
    fn forward(&mut self) {
        self.accept();
    }
}

#[itest(async)]
fn call_deferred_untyped(ctx: &crate::framework::TestContext) -> TaskHandle {
    let mut test_node = DeferredTestNode::new_alloc();
    ctx.scene_tree.clone().add_child(&test_node);

    // Called through Godot and therefore requires #[func] on the method.
    test_node.call_deferred("accept", &[]);

    let mut guard = test_node.bind_mut();
    guard.create_assertion_task()
}

#[itest(async)]
fn run_deferred_user_class(ctx: &crate::framework::TestContext) -> TaskHandle {
    let mut test_node = DeferredTestNode::new_alloc();
    ctx.scene_tree.clone().add_child(&test_node);

    let mut guard = test_node.bind_mut();

    // Explicitly check that this can be invoked on &mut T.
    let godot_class_ref: &mut DeferredTestNode = guard.deref_mut();
    godot_class_ref.run_deferred(DeferredTestNode::forward);

    guard.create_assertion_task()
}

#[itest(async)]
fn run_deferred_dyn(ctx: &crate::framework::TestContext) -> TaskHandle {
    let mut test_node = DeferredTestNode::new_alloc();
    ctx.scene_tree.clone().add_child(&test_node);

    // Explicitly check that this can be invoked on `DynGd` (NOT deref to &T).
    let mut dyn_gd: DynGd<Object, dyn ForwardTrait> = test_node.clone().into_dyn().upcast();
    dyn_gd.run_deferred(ForwardTrait::forward);

    let mut guard = test_node.bind_mut();
    guard.create_assertion_task()
}

#[itest(async)]
fn run_deferred_dyn_gd(ctx: &crate::framework::TestContext) -> TaskHandle {
    let mut test_node = DeferredTestNode::new_alloc();
    ctx.scene_tree.clone().add_child(&test_node);

    // Explicitly check that this can be invoked on `DynGd` (NOT deref to &T).
    let mut dyn_gd: DynGd<Object, dyn ForwardTrait> = test_node.clone().into_dyn().upcast();
    dyn_gd.run_deferred_gd(DeferredTestNode::accept_dyn_gd);

    let mut guard = test_node.bind_mut();
    guard.create_assertion_task()
}

#[itest(async)]
fn run_deferred_gd_user_class(ctx: &crate::framework::TestContext) -> TaskHandle {
    let mut test_node = DeferredTestNode::new_alloc();
    ctx.scene_tree.clone().add_child(&test_node);

    test_node.run_deferred_gd(DeferredTestNode::accept_gd);

    let mut guard = test_node.bind_mut();
    guard.create_assertion_task()
}

// Regression test for https://github.com/godot-rust/gdext/issues/1624: A `run_deferred[_gd]` closure must not run after `SceneTree` teardown,
// matching Godot's `call_deferred()` which drops such queued calls.
#[itest(async)]
fn run_deferred_skipped_when_exiting(ctx: &crate::framework::TestContext) -> TaskHandle {
    let mut test_node = DeferredTestNode::new_alloc();
    ctx.scene_tree.clone().add_child(&test_node);

    // The guard clears the exiting flag when dropped. The deferred call is flushed only a frame later, so we must keep the guard alive until
    // after that flush -- otherwise the flag would be cleared too early and the closure would run. Hence it is moved into the async task below.
    let guard = task::simulate_engine_exiting();

    let ran = Rc::new(Cell::new(false));
    let ran_setter = ran.clone();
    test_node.run_deferred_gd(move |_| ran_setter.set(true));

    // `test_completed` fires from `process()`, by which point the deferred queue has already been flushed (see other tests above).
    let mut guard_node = test_node.bind_mut();
    let run_test: SignalFuture<(StringName,)> = guard_node.signals().test_completed().to_future();
    drop(guard_node);

    task::spawn(async move {
        let _ = run_test.await;
        let was_run = ran.get();
        drop(guard); // Keep the flag set until after the deferred flush.
        assert!(!was_run, "run_deferred_gd() must not run during shutdown");
    })
}

#[itest(async)]
fn run_deferred_engine_class(ctx: &crate::framework::TestContext) -> TaskHandle {
    let mut test_node = DeferredTestNode::new_alloc();
    ctx.scene_tree.clone().add_child(&test_node);

    let mut node = test_node.clone().upcast::<Node>();
    node.run_deferred_gd(|mut that_node| that_node.set_name(ACCEPTED_NAME));

    let mut guard = test_node.bind_mut();
    guard.create_assertion_task()
}
