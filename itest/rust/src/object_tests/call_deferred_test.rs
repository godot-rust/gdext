/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::ops::DerefMut;

use godot::obj::WithBaseField;
use godot::prelude::*;
use godot::task::{SignalFuture, TaskHandle};

use crate::framework::itest;

const ACCEPTED_NAME: &str = "touched";

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

#[itest(async)]
fn call_deferred_untyped(ctx: &crate::framework::TestContext) -> TaskHandle {
    let mut test_node = DeferredTestNode::new_alloc();
    ctx.scene_tree.clone().add_child(&test_node);

    // Called through Godot and therefore requires #[func] on the method.
    test_node.call_deferred("accept", &[]);

    let mut gd_mut = test_node.bind_mut();
    gd_mut.create_assertion_task()
}

#[itest(async)]
fn call_deferred_godot_class(ctx: &crate::framework::TestContext) -> TaskHandle {
    let mut test_node = DeferredTestNode::new_alloc();
    ctx.scene_tree.clone().add_child(&test_node);

    let mut gd_mut = test_node.bind_mut();
    // Explicitly check that this can be invoked on &mut T.
    let godot_class_ref: &mut DeferredTestNode = gd_mut.deref_mut();
    godot_class_ref.apply_deferred(DeferredTestNode::accept);

    gd_mut.create_assertion_task()
}

#[itest(async)]
fn call_deferred_gd_user_class(ctx: &crate::framework::TestContext) -> TaskHandle {
    let mut test_node = DeferredTestNode::new_alloc();
    ctx.scene_tree.clone().add_child(&test_node);

    test_node.apply_deferred(DeferredTestNode::accept);

    let mut gd_mut = test_node.bind_mut();
    gd_mut.create_assertion_task()
}

#[itest(async)]
fn call_deferred_gd_engine_class(ctx: &crate::framework::TestContext) -> TaskHandle {
    let mut test_node = DeferredTestNode::new_alloc();
    ctx.scene_tree.clone().add_child(&test_node);

    let mut node = test_node.clone().upcast::<Node>();
    node.apply_deferred(|that_node| that_node.set_name(ACCEPTED_NAME));

    let mut gd_mut = test_node.bind_mut();
    gd_mut.create_assertion_task()
}
