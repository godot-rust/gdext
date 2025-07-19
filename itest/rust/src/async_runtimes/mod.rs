/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Async runtime integrations for different async runtimes.
//!
//! This module contains example implementations of the `AsyncRuntimeIntegration` trait
//! for popular async runtimes like tokio, async-std, smol, etc.
//!
//! The itest project demonstrates how to properly implement and register async runtime
//! integrations with gdext. Users can follow these patterns to integrate their preferred
//! async runtime.

pub mod tokio_runtime;

pub use tokio_runtime::TokioIntegration;
