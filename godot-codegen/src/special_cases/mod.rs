/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// Deliberately private -- all checks must go through `special_cases`.
mod codegen_special_cases;
#[allow(clippy::module_inception)]
mod special_cases;

// Content not in mod.rs to find the file quicker.
pub use special_cases::*;
