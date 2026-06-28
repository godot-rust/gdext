/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Integrates async rust code with the engine.
//!
//! This module contains:
//! - Implementations of [`Future`] for [`Signal`][crate::builtin::Signal] and [`TypedSignal`][crate::signal::TypedSignal].
//! - A way to [`spawn`] new async tasks by using the engine as the async runtime.

mod async_runtime;
mod futures;

// Public re-exports
pub use async_runtime::{TaskHandle, spawn};
pub use futures::{
    DynamicSend, FallibleSignalFuture, FallibleSignalFutureError, IntoDynamicSend, SignalFuture,
};

// For use in integration tests.
#[cfg(feature = "itest")]
mod reexport_test {
    pub use super::async_runtime::{
        EngineExitingGuard, has_godot_task_panicked, simulate_engine_exiting,
    };
    pub use super::futures::{SignalFutureResolver, create_test_signal_future_resolver};
}

#[cfg(feature = "itest")]
pub use reexport_test::*;

// Crate-local re-exports.
mod reexport_crate {
    pub(crate) use super::async_runtime::{
        await_point_dec, await_point_inc, cleanup, is_engine_exiting, mark_engine_exiting,
    };
    pub(crate) use super::futures::{ThreadConfined, impl_dynamic_send};
}

pub(crate) use reexport_crate::*;
