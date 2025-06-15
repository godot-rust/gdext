/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::framework::itest;
use crate::object_tests::deferred_call_test::TestState::{Accepted, Initial};
use godot::prelude::*;
use godot::task::{SignalFuture, TaskHandle};

#[derive(GodotConvert, Var, Export, Clone, PartialEq, Debug)]
#[godot(via = GString)]
enum TestState {
    Initial,
    Accepted,
}

#[derive(GodotClass)]
#[class(base=Node)]
struct DeferredTestNode {
    base: Base<Node>,
    state: TestState,
}

#[godot_api]
impl DeferredTestNode {
    #[signal]
    fn test_completed(state: TestState);

    #[func]
    fn accept(&mut self) {
        self.state = Accepted;
    }

    fn as_expectation_task(&self) -> TaskHandle {
        assert_eq!(Initial, self.state, "accept evaluated synchronously");

        let test_will_succeed: SignalFuture<(Variant,)> =
            Signal::from_object_signal(&self.to_gd(), "test_completed").to_future();
        godot::task::spawn(async move {
            let (final_state,) = test_will_succeed.await;
            let final_state: TestState = final_state.to();

            assert_eq!(Accepted, final_state);
        })
    }
}

#[godot_api]
impl INode for DeferredTestNode {
    fn init(base: Base<Self::Base>) -> Self {
        Self {
            base,
            state: Initial,
        }
    }

    fn process(&mut self, _delta: f64) {
        let args = vslice![self.state];
        self.base_mut().emit_signal("test_completed", args);
    }
}

#[itest(async)]
fn calls_method_names_deferred(ctx: &crate::framework::TestContext) -> TaskHandle {
    let mut test_node = DeferredTestNode::new_alloc();
    ctx.scene_tree.clone().add_child(&test_node);

    test_node.call_deferred("accept", &[]);

    let handle = test_node.bind().as_expectation_task();
    handle
}

#[itest(async)]
fn calls_closure_deferred(ctx: &crate::framework::TestContext) -> TaskHandle {
    let mut test_node = DeferredTestNode::new_alloc();
    ctx.scene_tree.clone().add_child(&test_node);

    test_node.apply_deferred(|mut this| this.bind_mut().accept());

    let handle = test_node.bind().as_expectation_task();
    handle
}
