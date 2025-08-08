/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Integrates async rust code with the engine.
//!
//! This module contains:
//! - Implementations of [`Future`](std::future::Future) for [`Signal`](crate::builtin::Signal) and [`TypedSignal`](crate::registry::signal::TypedSignal).
//! - A way to [`spawn`] new async tasks by using the engine as the async runtime.

mod async_runtime;
mod futures;

pub(crate) use async_runtime::cleanup;
pub use async_runtime::{spawn, TaskHandle};
pub(crate) use futures::{impl_dynamic_send, ThreadConfined};
pub use futures::{
    DynamicSend, FallibleSignalFuture, FallibleSignalFutureError, IntoDynamicSend, SignalFuture,
};

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Only exported for itest.

#[cfg(feature = "trace")]
pub use async_runtime::has_godot_task_panicked;
#[cfg(feature = "trace")]
pub use futures::{create_test_signal_future_resolver, SignalFutureResolver};
