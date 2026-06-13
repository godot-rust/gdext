/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::builtin::Variant;
use crate::r#gen::classes::Node;
use crate::meta::error::RpcError;
use crate::obj::{Gd, GodotClass, Inherits, WithBaseField};

/// The object a type-safe RPC is dispatched on, abstracting over the two ways RPCs are accessed: borrowed `&mut self` from within the class
/// ([`Internal`][Self::Internal]), or a `Gd` pointer from the outside ([`External`][Self::External]). Both converge on the same dispatch.
///
/// You do not construct this manually; it is held by an [`RpcBuilder`] created through the type-safe RPC API.
#[doc(hidden)]
pub enum UserRpcObject<'c, C: GodotClass> {
    /// Borrowed access from within the class, via `self.rpcs()`.
    Internal(&'c mut C),
    /// External access via `gd.rpcs()`, holding the `Gd` pointer directly.
    External(Gd<C>),
}

impl<'c, C> UserRpcObject<'c, C>
where
    C: WithBaseField + Inherits<Node>,
{
    /// Consumes [`Self`], calling the RPC with the provided arguments.
    pub fn call_rpc(self, name: &str, args: &[Variant]) -> Result<(), RpcError> {
        let error = match self {
            UserRpcObject::Internal(self_mut) => self_mut.to_gd().upcast::<Node>().rpc(name, args),
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
            UserRpcObject::Internal(self_mut) => {
                self_mut.to_gd().upcast::<Node>().rpc_id(id, name, args)
            }
            UserRpcObject::External(mut gd) => gd.upcast_mut::<Node>().rpc_id(id, name, args),
        };

        match error.try_into() {
            Ok(error) => Err(error),
            // We only fail to convert the error if it is `Error::OK`.
            Err(_) => Ok(()),
        }
    }
}
