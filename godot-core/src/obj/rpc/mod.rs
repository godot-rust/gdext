/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

pub use builder::*;
pub use rpc_object::*;

use crate::obj::GodotClass;

mod builder;
mod rpc_object;

/// Represents a collection of RPCs that can be constructed with a [`UserRpcObject`].
pub trait RpcCollection<'c, C>
where
    C: GodotClass,
{
    #[doc(hidden)]
    fn from_user_rpc_object(object: UserRpcObject<'c, C>) -> Self;
}
