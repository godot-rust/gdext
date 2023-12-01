/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::init::{gdextension, ExtensionLibrary};

mod benchmarks;
mod builtin_tests;
mod common;
mod engine_tests;
mod framework;
mod object_tests;
mod register_tests;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Entry point

#[gdextension(entry_point=itest_init)]
unsafe impl ExtensionLibrary for framework::IntegrationTests {}
