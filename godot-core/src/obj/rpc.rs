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

/// Holds:
/// * A [`UserRpcObject`], which the represented RPC is called on.
/// * A [`&str`], which represents the name of an RPC,
/// * A [`Vec`] of [`Variants`] that is a list of parameters passed to the RPC when called.
pub struct GenericRpcBuilder<'c, C: GodotClass> {
    object: UserRpcObject<'c, C>,
    rpc_name: &'c str,
    parameters: Vec<Variant>,
}

impl<'c, C: GodotClass> GenericRpcBuilder<'c, C> {
    pub fn new(object: UserRpcObject<'c, C>, rpc_name: &'c str, parameters: Vec<Variant>) -> Self {
        Self {
            object,
            rpc_name,
            parameters,
        }
    }
}

impl<'c, C> GenericRpcBuilder<'c, C>
where
    C: WithBaseField + Inherits<Node>,
{
    pub fn call(self) {
        self.object.call_rpc(self.rpc_name, &self.parameters);
    }

    pub fn call_id(self, id: i64) {
        self.object.call_rpc_id(self.rpc_name, id, &self.parameters);
    }
}

/// Holds either an [`Internal`](Self::Internal) reference of `&mut C` or an [`External`](Self::External) [`Gd`] pointer to `C`.
///
/// If `C` implements [`WithBaseField`] and [`Inherits`] [`Node`], the [`call_rpc()`](Self::call_rpc()) and
/// [`call_rpc_id()`](Self::call_rpc_id()) methods are provided for calling RPCs on the held node.
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
{
    /// Consumes [`Self`], calling the given RPC with `parameters`
    pub fn call_rpc(self, name: &str, parameters: &[Variant]) {
        match self {
            UserRpcObject::Internal(self_mut) => {
                let mut gd = <C as WithBaseField>::to_gd(self_mut);
                gd.upcast_mut::<Node>().rpc(name, parameters);
            }
            UserRpcObject::External(mut gd) => {
                gd.upcast_mut::<Node>().rpc(name, parameters);
            }
        }
    }

    /// Consumes [`Self`], calling the given RPC, on `id`, with `parameters`
    pub fn call_rpc_id(self, name: &str, id: i64, parameters: &[Variant]) {
        match self {
            UserRpcObject::Internal(self_mut) => {
                let mut gd = <C as WithBaseField>::to_gd(self_mut);
                gd.upcast_mut::<Node>().rpc_id(id, name, parameters);
            }
            UserRpcObject::External(mut gd) => {
                gd.upcast_mut::<Node>().rpc_id(id, name, parameters);
            }
        }
    }
}

/// Represents a collection of RPCs that can be constructed with a [`UserRpcObject`]
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

    /// Returns [`Self::Collection`], which generally holds a reference to [`Self`]
    fn rpcs(&'c mut self) -> Self::Collection;
}

impl<'c, C> WithUserRpcs<'c, C> for Gd<C>
where
    C: Inherits<Node> + WithUserRpcs<'c, C>,
{
    type Collection = <C as WithUserRpcs<'c, C>>::Collection;

    /// Returns [`Self::Collection`], which generally holds a [`Gd`] pointer to [`Self`]
    fn rpcs(&'c mut self) -> Self::Collection {
        Self::Collection::from_user_rpc_object(UserRpcObject::External(self.clone()))
    }
}
