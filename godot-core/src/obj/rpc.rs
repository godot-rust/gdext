/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::builtin::Variant;
use crate::r#gen::classes::Node;
use crate::r#gen::virtuals::RefCounted::Gd;
use crate::obj::{GodotClass, Inherits, WithBaseField};

/// Represents an RPC, and the object that it can be called on, and is usually obtained through the type-safe RPC API. See
/// the [relevant section]() in the book for more information about type-safe RPC calls.
pub struct RpcBuilder<'c, C: GodotClass> {
    object: UserRpcObject<'c, C>,
    rpc_name: &'c str,
    parameters: Vec<Variant>,
}

impl<'c, C: GodotClass> RpcBuilder<'c, C> {
    pub fn new(object: UserRpcObject<'c, C>, rpc_name: &'c str, parameters: Vec<Variant>) -> Self {
        Self {
            object,
            rpc_name,
            parameters,
        }
    }
}

impl<'c, C> RpcBuilder<'c, C>
where
    C: WithBaseField + Inherits<Node>,
    C::Base: Inherits<Node>,
{
    pub fn call(self) {
        self.object.call_rpc(self.rpc_name, &self.parameters);
    }

    pub fn call_id(self, id: i64) {
        self.object.call_rpc_id(self.rpc_name, id, &self.parameters);
    }
}

/// Represents an object that RPCs can be called on.
///
/// You generally do not need to create this manually, rather it used internally by the type-safe RPC API.
pub enum UserRpcObject<'c, C: GodotClass> {
    /// Holds a mutabel reference to the [`GodotClass`]
    Internal(&'c mut C),
    /// Holds a [`Gd`] pointer to the [`GodotClass`]
    External(Gd<C>),
}

// TODO: forward errors from RPC dispatch
impl<'c, C> UserRpcObject<'c, C>
where
    C: WithBaseField + Inherits<Node>,
    C::Base: Inherits<Node>,
{
    /// Consumes [`Self`], calling the given RPC with `parameters`.
    pub fn call_rpc(self, name: &str, parameters: &[Variant]) {
        match self {
            UserRpcObject::Internal(self_mut) => {
                WithBaseField::base_mut(self_mut)
                    .upcast_mut::<Node>()
                    .rpc(name, parameters);
            }
            UserRpcObject::External(mut gd) => {
                gd.upcast_mut::<Node>().rpc(name, parameters);
            }
        }
    }

    /// Consumes [`Self`], calling the given RPC, on `id`, with `parameters`.
    pub fn call_rpc_id(self, name: &str, id: i64, parameters: &[Variant]) {
        match self {
            UserRpcObject::Internal(self_mut) => {
                WithBaseField::base_mut(self_mut)
                    .upcast_mut::<Node>()
                    .rpc_id(id, name, parameters);
            }
            UserRpcObject::External(mut gd) => {
                gd.upcast_mut::<Node>().rpc_id(id, name, parameters);
            }
        }
    }
}

/// Represents a collection of RPCs that can be constructed with a [`UserRpcObject`].
pub trait RpcCollection<'c, C>
where
    C: GodotClass,
{
    /// Construct [`Self`] from [`UserRpcObject`]
    fn from_user_rpc_object(object: UserRpcObject<'c, C>) -> Self;
}

/// Represents an object, generally a node, that can provide a [collection](RpcCollection) of RPCs available on this object.
pub trait WithUserRpcs<'c, C>
where
    C: GodotClass,
{
    type Collection: RpcCollection<'c, C>;

    /// Returns [`Self::Collection`], which generally holds a reference to [`Self`].
    fn rpcs(&'c mut self) -> Self::Collection;
}

impl<'c, C> WithUserRpcs<'c, C> for Gd<C>
where
    C: Inherits<Node> + WithUserRpcs<'c, C>,
{
    type Collection = <C as WithUserRpcs<'c, C>>::Collection;

    /// Returns `Self::Collection`, which generally holds a [`Gd`] pointer to [`Self`].
    fn rpcs(&'c mut self) -> Self::Collection {
        Self::Collection::from_user_rpc_object(UserRpcObject::External(self.clone()))
    }
}
