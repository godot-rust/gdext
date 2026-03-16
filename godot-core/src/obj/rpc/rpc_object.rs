/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::builtin::Variant;
use crate::r#gen::classes::Node;
use crate::r#gen::virtuals::RefCounted::Gd;
use crate::meta::error::RpcError;
use crate::obj::{GodotClass, Inherits, WithBaseField};

/// Represents an object that RPCs can be called on.
///
/// You generally do not need to create this manually, rather it used internally by the type-safe RPC API.
pub enum UserRpcObject<'c, C: GodotClass> {
    /// Used in [`Self::call_rpc`] and [`Self::call_rpc_id`] to access the base class and call an RPC.
    Internal(&'c mut C),
    /// RPCs are called on this object directly in [`Self::call_rpc`] and [`Self::call_rpc_id`].
    External(Gd<C>),
}

impl<'c, C> UserRpcObject<'c, C>
where
    C: WithBaseField + Inherits<Node>,
{
    /// Consumes [`Self`], calling the RPC with the provided arguments.
    pub fn call_rpc(self, name: &str, args: &[Variant]) -> Result<(), RpcError> {
        let error = match self {
            UserRpcObject::Internal(self_mut) => self_mut
                .base_mut()
                .clone()
                .owned_cast::<Node>()
                .expect("This is a bug, please report it.")
                .rpc(name, args),
            UserRpcObject::External(mut gd) => gd.upcast_mut::<Node>().rpc(name, args),
        };

        match error.try_into() {
            Ok(error) => Err(error),
            // We only fail to convert the error if it is `Error::OK`.
            Err(_) => Ok(()),
        }
    }

    /// Consumes [`Self`], calling the RPC by a specific ID with the provided arguments.
    pub fn call_rpc_id(self, name: &str, id: i64, args: &[Variant]) -> Result<(), RpcError> {
        let error = match self {
            UserRpcObject::Internal(self_mut) => self_mut
                .base_mut()
                .clone()
                .owned_cast::<Node>()
                .expect("This is a bug, please report it.")
                .rpc_id(id, name, args),
            UserRpcObject::External(mut gd) => gd.upcast_mut::<Node>().rpc_id(id, name, args),
        };

        match error.try_into() {
            Ok(error) => Err(error),
            // We only fail to convert the error if it is `Error::OK`.
            Err(_) => Ok(()),
        }
    }
}
