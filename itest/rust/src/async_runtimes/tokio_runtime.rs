/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Example implementation of AsyncRuntimeIntegration for Tokio
//!
//! This is a demonstration of how to properly implement and integrate a tokio runtime
//! with gdext's async system. This serves as a reference implementation that users
//! can follow to integrate their preferred async runtime (async-std, smol, etc.).
//!
//! The itest project demonstrates:
//! 1. How to implement the `AsyncRuntimeIntegration` trait
//! 2. How to register the runtime with gdext
//! 3. How to use async functions in Godot classes
//! 4. How to handle runtime lifecycle and context management

use godot::task::AsyncRuntimeIntegration;
use std::any::Any;

/// Minimal tokio runtime integration for gdext
///
/// Users need to implement both `create_runtime()` and `with_context()` for the integration.
pub struct TokioIntegration;

impl AsyncRuntimeIntegration for TokioIntegration {
    type Handle = tokio::runtime::Handle;

    fn create_runtime() -> Result<(Box<dyn Any + Send + Sync>, Self::Handle), String> {
        // Create a multi-threaded runtime with proper configuration
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .thread_name("gdext-tokio")
            .worker_threads(2)
            .build()
            .map_err(|e| format!("Failed to create tokio runtime: {e}"))?;

        let handle = runtime.handle().clone();

        // Return both - gdext manages the lifecycle automatically
        Ok((Box::new(runtime), handle))
    }

    fn with_context<R>(handle: &Self::Handle, f: impl FnOnce() -> R) -> R {
        // Enter the tokio runtime context to make it current
        let _guard = handle.enter();
        f()
    }
}
