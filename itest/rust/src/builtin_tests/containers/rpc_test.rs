/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::meta::error::RpcError;
use godot::prelude::*;
use godot::test::itest;

use crate::framework::TestContext;

#[derive(GodotClass)]
#[class(init, base = Node)]
pub struct RpcCallableNode {
    base: Base<Node>,
}

#[godot_api]
impl INode for RpcCallableNode {}

#[godot_api]
impl RpcCallableNode {
    #[rpc]
    pub fn say_hello_world(&mut self) {
        godot_print!("hello, world");
    }

    #[rpc(call_local)]
    pub fn say_hello_world_everywhere(&mut self) {
        godot_print!("hello, world");
    }

    #[rpc]
    pub fn say_hello_to(&mut self, to: String) {
        godot_print!("hello, {to}");
    }

    #[rpc]
    pub fn say_number(&self, number: i32) {
        godot_print!("{number}");
    }

    #[rpc]
    #[func(rename = renamed_rpc_function)]
    pub fn rpc_function(&self) {}
}

#[itest]
fn type_safe_rpc_test(context: &TestContext) {
    // This peer id doesn't, or at least shouldn't, exist.
    const NON_EXISTENT_ID: i64 = 100000;

    let mut node = RpcCallableNode::new_alloc();

    let mut root = context.scene_tree.clone();

    // Before we add the node to the tree, RPCs will fail.
    assert_eq!(
        Err(RpcError::Unconfigured),
        node.bind_mut().rpcs().say_hello_world().call()
    );
    // We'll still fail with `Unconfigured` here even if the ID passed here doesn't belong to an existing node.
    assert_eq!(
        Err(RpcError::Unconfigured),
        node.bind_mut()
            .rpcs()
            .say_hello_world()
            .call_id(NON_EXISTENT_ID)
    );

    root.add_child(&node);

    #[cfg(feature = "codegen-full")]
    {
        assert_eq!(Ok(()), node.rpcs().say_hello_world().call());

        let arg = "godot".to_string();
        assert_eq!(Ok(()), node.rpcs().say_hello_to(arg.clone()).call());
        assert_eq!(Ok(()), node.bind_mut().rpcs().say_hello_to(arg).call());
        assert_eq!(Ok(()), node.rpcs().say_number(3).call());

        // Calling the renamed function should work.
        assert_eq!(Ok(()), node.rpcs().rpc_function().call());

        // Even though the node with the ID doesn't exist, godot will still report the RPC as a success.
        assert_eq!(Ok(()), node.rpcs().say_number(5).call_id(NON_EXISTENT_ID));

        // Gets the peer ID for this node.
        let node_id = node
            .get_multiplayer()
            .expect("Multiplayer should exist")
            .get_unique_id() as i64;
        // The `say_hello_world` RPC does not define `call_local` meaning it will error when called upon itself.
        assert_eq!(
            Err(RpcError::InvalidArguments),
            node.rpcs().say_hello_world().call_id(node_id)
        );
        // The `say_hello_world_everywhere` RPC does define `call_local` so it won't error when called upon itself.
        assert_eq!(
            Ok(()),
            node.rpcs().say_hello_world_everywhere().call_id(node_id)
        );
    }
    root.remove_child(&node);
    node.free();
}
