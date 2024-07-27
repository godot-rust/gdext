/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

mod constant_test;
mod conversion_test;
mod derive_variant_test;
mod func_test;
mod gdscript_ffi_test;
mod keyword_parameters_test;
mod option_ffi_test;
mod var_test;

#[cfg(since_api = "4.3")]
mod func_virtual_test;

pub use gdscript_ffi_test::gen_ffi;
