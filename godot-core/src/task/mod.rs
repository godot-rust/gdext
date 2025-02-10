/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

mod futures;

pub use futures::{FallibleSignalFuture, FallibleSignalFutureError, SignalFuture};

// Only exported for itest.
#[cfg(feature = "trace")]
pub use futures::SignalFutureResolver;
