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

// There's no way to check if the method was registered as an RPC.
// We could set up a multiplayer environment to test this in practice, but that would be a lot of work.
#[itest]
fn node_enters_tree() {
    let node = RpcTest::new_alloc();

    // Registering is done in `UserClass::__before_ready()`, and it requires a multiplayer API to exist.
    let mut scene_tree = Engine::singleton()
        .get_main_loop()
        .unwrap()
        .cast::<SceneTree>();
    scene_tree.set_multiplayer(MultiplayerApi::create_default_interface().as_ref());

    let mut root = scene_tree.get_root().unwrap();
    root.add_child(&node);
    root.remove_child(&node);
    node.free();
}
