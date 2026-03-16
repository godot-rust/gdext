/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// TODO: Investigate reducing the type bounds if possible.

use crate::r#gen::classes::Node;
use crate::r#gen::virtuals::RefCounted::Gd;
use crate::obj::{GodotClass, Inherits, WithBaseField};

pub enum UserRpcObject<'c, C: GodotClass> {
    Internal(&'c mut C),
    External(Gd<C>),
}

impl<'c, C> UserRpcObject<'c, C>
where
    C: GodotClass + WithBaseField + Inherits<Node>,
{
    // TODO: should this be #[inline]?
    #[doc(hidden)]
    pub fn with_object_mut(&mut self, f: impl FnOnce(&mut Node)) {
        match self {
            UserRpcObject::Internal(self_mut) => {
                let mut gd = <C as WithBaseField>::to_gd(self_mut);
                f(gd.upcast_mut::<Node>());
            }
            UserRpcObject::External(gd) => f(gd.upcast_mut::<Node>()),
        }
    }
}

pub trait RpcCollection<'c, C>
where
    C: GodotClass + WithBaseField + Inherits<Node>,
{
    fn from_user_rpc_object(object: UserRpcObject<'c, C>) -> Self;
}

pub trait WithUserRpcs<'c, C>
where
    C: GodotClass + WithBaseField + Inherits<Node> + WithUserRpcs<'c, C>,
{
    type Collection: RpcCollection<'c, C>;

    fn rpcs(&'c mut self) -> Self::Collection;
}

impl<'c, C> WithUserRpcs<'c, C> for Gd<C>
where
    C: GodotClass + WithBaseField + Inherits<Node> + WithUserRpcs<'c, C>,
{
    type Collection = <C as WithUserRpcs<'c, C>>::Collection;

    fn rpcs(&'c mut self) -> Self::Collection {
        Self::Collection::from_user_rpc_object(UserRpcObject::External(self.clone()))
    }
}
