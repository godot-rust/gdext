/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::builtin::{StringName, VarDictionary};
use crate::classes::multiplayer_api::RpcMode;
use crate::classes::multiplayer_peer::TransferMode;
use crate::classes::Node;
use crate::meta::{AsArg, ToGodot};
use crate::{arg_into_ref, vdict};

/// Configuration for a remote procedure call, used with `#[rpc(config = ...)]`.
///
/// Check documentation of the [`#[rpc]` attribute](attr.godot_api.html#rpc-attributes) for usage.
///
/// See also [Godot `@rpc` keyword](https://docs.godotengine.org/en/stable/tutorials/networking/high_level_multiplayer.html#remote-procedure-calls).
#[derive(Copy, Clone, Debug)]
pub struct RpcConfig {
    pub rpc_mode: RpcMode,
    pub transfer_mode: TransferMode,
    pub call_local: bool,
    pub channel: u32,
}

impl Default for RpcConfig {
    fn default() -> Self {
        Self {
            rpc_mode: RpcMode::AUTHORITY,
            transfer_mode: TransferMode::UNRELIABLE,
            call_local: false,
            channel: 0,
        }
    }
}

impl RpcConfig {
    /// Register `method` as a remote procedure call on `node`.
    pub fn configure_node(self, node: &mut Node, method_name: impl AsArg<StringName>) {
        arg_into_ref!(method_name);
        node.rpc_config(method_name, &self.to_dictionary().to_variant());
    }

    /// Returns an untyped `Dictionary` populated with the values required for a call to [`Node::rpc_config()`].
    pub fn to_dictionary(&self) -> VarDictionary {
        vdict! {
            "rpc_mode": self.rpc_mode,
            "transfer_mode": self.transfer_mode,
            "call_local": self.call_local,
            "channel": self.channel,
        }
    }
}
