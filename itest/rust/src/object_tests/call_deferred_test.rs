/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
use crate::framework::itest;
use godot::obj::WithBaseField;
use godot::prelude::*;
use godot::task::{SignalFuture, TaskHandle};

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
