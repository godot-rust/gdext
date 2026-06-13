/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::classes::multiplayer_api::RpcMode;
use godot::classes::multiplayer_peer::TransferMode;
use godot::classes::{Engine, MultiplayerApi};
use godot::obj::Singleton;
use godot::prelude::*;
use godot::register::RpcConfig;
use godot::test::itest;

#[derive(GodotClass)]
#[class(init, base = Node2D)]
pub struct RpcTest {
    base: Base<Node2D>,
}

const CACHED_CFG: RpcConfig = RpcConfig {
    rpc_mode: RpcMode::AUTHORITY,
    transfer_mode: TransferMode::RELIABLE,
    call_local: false,
    channel: 1,
};

fn provide_cfg() -> RpcConfig {
    RpcConfig {
        transfer_mode: TransferMode::RELIABLE,
        channel: 1,
        ..Default::default()
    }
}

#[godot_api]
impl RpcTest {
    #[rpc]
    pub fn default_args(&mut self) {}

    #[rpc(any_peer)]
    pub fn arg_any_peer(&mut self) {}

    #[rpc(authority)]
    pub fn arg_authority(&mut self) {}

    #[rpc(reliable)]
    pub fn arg_reliable(&mut self) {}

    #[rpc(unreliable)]
    pub fn arg_unreliable(&mut self) {}

    #[rpc(unreliable_ordered)]
    pub fn arg_unreliable_ordered(&mut self) {}

    #[rpc(call_local)]
    pub fn arg_call_local(&mut self) {}

    #[rpc(call_remote)]
    pub fn arg_call_remote(&mut self) {}

    #[rpc(channel = 2)]
    pub fn arg_channel(&mut self) {}

    #[rpc(any_peer, reliable, call_remote, channel = 2)]
    pub fn all_args(&mut self) {}

    #[rpc(reliable, any_peer)]
    #[func]
    pub fn args_func(&mut self) {}

    #[rpc(unreliable)]
    #[func(gd_self)]
    pub fn args_func_gd_self(_this: Gd<Self>) {}

    #[rpc(config = CACHED_CFG)]
    pub fn arg_config_const(&mut self) {}

    #[rpc(config = provide_cfg())]
    pub fn arg_config_fn(&mut self) {}
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Tests

/// Verifies that a class with many `#[rpc]` attribute variants can be added to a tree without panicking.
/// RPC registration runs inside `__before_ready()` and requires a `MultiplayerApi` to be present.
///
/// Unlike `RpcCallableNode` (in `builtin_tests::containers::rpc_test`), `RpcTest` has no `impl INode` block and no `OnReady` fields, so the
/// derive-macro's default `_ready` is the only thing that can trigger registration -- this is the regression test for that path. Dispatch
/// behavior itself (`call_local`, argument roundtrips) is covered there; here we only assert that registration happened at all.
#[itest]
fn rpc_registration_all_attr_variants() {
    let node = RpcTest::new_alloc();

    let mut scene_tree = Engine::singleton()
        .get_main_loop()
        .unwrap()
        .cast::<SceneTree>();
    scene_tree.set_multiplayer(MultiplayerApi::create_default_interface().as_ref());

    #[cfg(since_api = "4.7")]
    let mut root = scene_tree.get_root();
    #[cfg(before_api = "4.7")]
    let mut root = scene_tree.get_root().unwrap();
    root.add_child(&node);

    #[cfg(feature = "codegen-full")]
    {
        let own_id = node
            .get_multiplayer()
            .expect("multiplayer should exist")
            .get_unique_id() as i64;

        // `arg_call_local` has `call_local = true`, so targeting own peer ID succeeds -- but only if the RPC was actually registered.
        // Without registration, Godot rejects the call with `InvalidArguments`, which is what this assertion guards against.
        assert_eq!(node.rpcs().arg_call_local().call_id(own_id), Ok(()));
    }

    root.remove_child(&node);
    node.free();
}
