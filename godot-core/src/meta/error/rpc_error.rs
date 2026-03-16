/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::error::Error;
use std::fmt;

use crate::global;
use crate::obj::EngineEnum;

#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum RpcError {
    /// The callee node was not connected to a server.
    NotConnected,
    /// The RPC was called with incorrect arguments.
    ///
    /// _If this error occurs through the type-safe API, it is probably a bug._
    InvalidArguments,
    /// The callee node's multiplayer could not be fetched. This is likely to happen when the node has yet to be added to the
    /// tree.
    Unconfigured,
    /// An error that is unlikely to come from interacting with RPCs.
    Unrelated(global::Error),
}

impl Error for RpcError {}

impl fmt::Display for RpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RpcError::NotConnected => write!(
                f,
                "The node that this RPC was called on is not connected to a server."
            ),
            RpcError::InvalidArguments => write!(
                f,
                "The arguments passed to the RPC method do not match what was expected."
            ),
            RpcError::Unconfigured => write!(
                f,
                "Could not get the multiplayer field for the node that this RPC was called on."
            ),
            RpcError::Unrelated(error) => write!(
                f,
                "An error unrelated to RPCs has occurred. Check the Godot error documentation \
                (https://docs.godotengine.org/en/stable/classes/class_@globalscope.html#enum-globalscope-error) for the error \
                at ordinal '{}'. Error message: {error:?}",
                error.ord(),
            ),
        }
    }
}

impl TryFrom<global::Error> for RpcError {
    type Error = ();

    fn try_from(error: global::Error) -> Result<Self, Self::Error> {
        match error {
            global::Error::ERR_UNCONFIGURED => Ok(RpcError::Unconfigured),
            global::Error::ERR_INVALID_PARAMETER => Ok(RpcError::InvalidArguments),
            global::Error::ERR_CONNECTION_ERROR => Ok(RpcError::NotConnected),
            global::Error::OK => Err(()),
            _ => Ok(RpcError::Unrelated(error)),
        }
    }
}
