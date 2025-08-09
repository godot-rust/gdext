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

// Public re-exports
pub use async_runtime::{spawn, TaskHandle};
pub use futures::{
    DynamicSend, FallibleSignalFuture, FallibleSignalFutureError, IntoDynamicSend, SignalFuture,
};

// For use in integration tests.
#[cfg(feature = "trace")]
mod reexport_test {
    pub use super::async_runtime::has_godot_task_panicked;
    pub use super::futures::{create_test_signal_future_resolver, SignalFutureResolver};
}

#[cfg(feature = "trace")]
pub use reexport_test::*;

// Crate-local re-exports.
mod reexport_crate {
    pub(crate) use super::async_runtime::cleanup;
    pub(crate) use super::futures::{impl_dynamic_send, ThreadConfined};
}

pub(crate) use reexport_crate::*;
