/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::init::{gdextension, ExtensionLibrary};

mod framework;

mod array_test;
mod base_test;
mod basis_test;
mod builtin_test;
mod callable_test;
mod codegen_test;
mod color_test;
mod derive_variant;
mod dictionary_test;
mod enum_test;
mod func_test;
mod gdscript_ffi_test;
mod init_test;
mod native_structures_test;
mod node_test;
mod object_test;
mod option_ffi_test;
mod packed_array_test;
mod plane_test;
mod projection_test;
mod property_test;
mod quaternion_test;
mod rect2_test;
mod rect2i_test;
mod registration;
mod rid_test;
mod signal_test;
mod singleton_test;
mod string;
mod transform2d_test;
mod transform3d_test;
mod utilities_test;
mod variant_test;
mod virtual_methods_test;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// API for test cases

#[gdextension(entry_point=itest_init)]
unsafe impl ExtensionLibrary for framework::IntegrationTests {}
