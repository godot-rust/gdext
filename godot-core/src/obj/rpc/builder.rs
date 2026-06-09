/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::builtin::Variant;
use crate::r#gen::classes::Node;
use crate::meta::error::RpcError;
use crate::obj::rpc::{GodotClass, UserRpcObject};
use crate::obj::{Inherits, WithBaseField};

/// Represents an RPC, and the object that it can be called on, and is usually obtained through the type-safe RPC API. See
/// the [relevant section]() in the book for more information about type-safe RPC calls.
pub struct RpcBuilder<'c, C: GodotClass> {
    object: UserRpcObject<'c, C>,
    rpc_name: &'c str,
    arguments: Vec<Variant>,
}

impl<'c, C: GodotClass> RpcBuilder<'c, C> {
    pub fn new(object: UserRpcObject<'c, C>, rpc_name: &'c str, arguments: Vec<Variant>) -> Self {
        Self {
            object,
            rpc_name,
            arguments,
        }
    }
}

impl<'c, C> RpcBuilder<'c, C>
where
    C: WithBaseField + Inherits<Node>,
{
    pub fn call(self) -> Result<(), RpcError> {
        self.object.call_rpc(self.rpc_name, &self.arguments)
    }

    pub fn call_id(self, id: i64) -> Result<(), RpcError> {
        self.object.call_rpc_id(self.rpc_name, id, &self.arguments)
    }
}
