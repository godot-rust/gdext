/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

pub use builder::*;

mod builder;
mod rpc_object;

// `UserRpcObject` is plumbing for macro-generated code, never named by users; kept off the public path. Used internally via the crate path,
// and by macro expansion through the `godot::private` bridge below. Mirrors `signal::priv_re_export`.
pub(crate) use rpc_object::UserRpcObject;

// Bridge for `godot::private` (proc-macro internals).
#[doc(hidden)]
pub mod priv_re_export {
    pub use super::rpc_object::UserRpcObject;
}
