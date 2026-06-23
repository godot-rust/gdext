/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::meta::error::RpcError;
use godot::prelude::*;
use godot::test::itest;

#[cfg(feature = "codegen-full")]
use crate::framework::TestContext;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Class under test

#[derive(GodotClass)]
#[class(init, base = Node)]
pub struct RpcCallableNode {
    /// How many times a `call_local` RPC body has executed on this object.
    call_count: usize,
    /// Last value received by [`rpc_local_with_arg`][Self::rpc_local_with_arg]; checks POD argument roundtrip.
    last_arg: i32,
    /// Last string received by [`rpc_local_with_string`][Self::rpc_local_with_string]; checks heap-allocated argument roundtrip.
    last_text: GString,
    base: Base<Node>,
}

#[godot_api]
impl INode for RpcCallableNode {}

#[godot_api]
impl RpcCallableNode {
    /// No `call_local` -- Godot rejects targeting own peer ID with this RPC.
    #[rpc]
    pub fn rpc_remote_only(&mut self) {
        self.call_count += 1;
    }

    /// With `call_local` -- body executes locally when targeting own peer ID.
    #[rpc(call_local)]
    pub fn rpc_local_also(&mut self) {
        self.call_count += 1;
    }

    /// `call_local` with a POD argument -- verifies `i32` roundtrip through Variant serialization.
    #[rpc(call_local)]
    pub fn rpc_local_with_arg(&mut self, value: i32) {
        self.call_count += 1;
        self.last_arg = value;
    }

    /// `call_local` with a heap-allocated argument -- verifies `GString` roundtrip through Variant serialization.
    #[rpc(call_local)]
    pub fn rpc_local_with_string(&mut self, text: GString) {
        self.call_count += 1;
        self.last_text = text;
    }

    /// Verifies that a function renamed via `#[func(rename = ...)]` is still reachable by its Rust name.
    #[rpc]
    #[func(rename = renamed_rpc_function)]
    pub fn rpc_function(&self) {}
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Tests

/// Both the External path (`Gd::rpcs()`) and Internal path (`self.rpcs()`) must return `Unconfigured`
/// before the node is part of a scene tree with a resolvable `MultiplayerApi`.
#[itest]
fn rpc_unconfigured_before_tree() {
    let mut node = RpcCallableNode::new_alloc();
    const PEER_ID: i64 = 100_000;

    // External path: call() and call_id() both fail.
    assert_eq!(
        node.rpcs().rpc_local_also().call(),
        Err(RpcError::Unconfigured)
    );
    assert_eq!(
        node.rpcs().rpc_local_also().call_id(PEER_ID),
        Err(RpcError::Unconfigured)
    );

    // Internal path: same failures, different code path.
    assert_eq!(
        node.bind_mut().rpcs().rpc_local_also().call(),
        Err(RpcError::Unconfigured)
    );
    assert_eq!(
        node.bind_mut().rpcs().rpc_local_also().call_id(PEER_ID),
        Err(RpcError::Unconfigured)
    );

    node.free();
}

/// Verifies the `call_local` flag controls whether the local body runs when targeting own peer ID, that POD and
/// heap-allocated arguments survive the Variant roundtrip, and that both External and Internal entry points dispatch.
///
/// A `call_local` body runs synchronously during `call_id()` and mutates `self` through the dispatcher's `&mut self`,
/// which also proves the `GdCell` is writeable from inside an External-path callback.
///
/// Requires `codegen-full` because RPC registration (`Node::rpc_config`) is only wired up under that feature.
#[cfg(feature = "codegen-full")]
#[itest]
fn rpc_call_local_side_effects(context: &TestContext) {
    let mut node = RpcCallableNode::new_alloc();
    context.scene_tree.clone().add_child(&node);
    let own_id = node
        .get_multiplayer()
        .expect("multiplayer should exist")
        .get_unique_id() as i64;

    // Without call_local, targeting own peer ID is rejected by Godot; body does not execute.
    assert_eq!(
        node.rpcs().rpc_remote_only().call_id(own_id),
        Err(RpcError::InvalidArguments)
    );
    assert_eq!(node.bind().call_count, 0);

    // With call_local, targeting own peer ID succeeds and the body runs synchronously, mutating self.
    assert_eq!(node.rpcs().rpc_local_also().call_id(own_id), Ok(()));
    assert_eq!(node.bind().call_count, 1);

    // POD argument roundtrip through Variant serialization.
    assert_eq!(node.rpcs().rpc_local_with_arg(42).call_id(own_id), Ok(()));
    assert_eq!(node.bind().call_count, 2);
    assert_eq!(node.bind().last_arg, 42);

    // Heap-allocated argument roundtrip through Variant serialization.
    assert_eq!(
        node.rpcs().rpc_local_with_string("godot").call_id(own_id),
        Ok(())
    );
    assert_eq!(node.bind().call_count, 3);
    assert_eq!(node.bind().last_text.to_string(), "godot");

    // Internal path (`self.rpcs()` via bind_mut): a non-`call_local` broadcast does not execute locally, so dispatching
    // while `bind_mut()` is held does not re-enter the GdCell. Returns Ok without running the body (count unchanged).
    assert_eq!(node.bind_mut().rpcs().rpc_remote_only().call(), Ok(()));
    assert_eq!(node.bind().call_count, 3);

    // Renamed function is reachable by its Rust name.
    assert_eq!(node.rpcs().rpc_function().call(), Ok(()));

    // Godot does not validate peer existence at dispatch time; non-existent peer ID returns Ok.
    assert_eq!(node.rpcs().rpc_local_also().call_id(100_000), Ok(()));

    context.scene_tree.clone().remove_child(&node);
    node.free();
}

/// Internal path (`self.rpcs()`) + `call_local` targeting own peer: the RPC body re-enters the `GdCell` while the outer `bind_mut()` borrow is
/// still held. Must not panic.
///
/// Regression test, see https://github.com/godot-rust/gdext/pull/1643.
#[cfg(feature = "codegen-full")]
#[itest]
fn rpc_internal_call_local_reentrant(context: &TestContext) {
    let mut node = RpcCallableNode::new_alloc();
    context.scene_tree.clone().add_child(&node);
    let own_id = node
        .get_multiplayer()
        .expect("multiplayer should exist")
        .get_unique_id() as i64;

    {
        let mut guard = node.bind_mut();
        assert_eq!(guard.rpcs().rpc_local_also().call_id(own_id), Ok(()));
    }
    assert_eq!(node.bind().call_count, 1);

    context.scene_tree.clone().remove_child(&node);
    node.free();
}
