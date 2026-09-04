/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

mod base_test;
mod bitfield_ops_test;
mod call_deferred_test;
mod class_id_test;
mod class_rename_test;
mod dyn_gd_test;
mod dynamic_call_test;
mod enum_test;
mod gd_duplicate_test;
// `get_property_list` is only supported in Godot 4.3+.
#[cfg(since_api = "4.3")]
mod get_property_list_test;
mod init_stage_test;
mod object_arg_test;
mod object_swap_test;
mod object_test;
mod oneditor_test;
mod onready_test;
mod phantom_var_test;
mod property_template_test;
mod property_test;
mod reentrant_test;
// Before Godot 4.4, an object destroyed inside its own call is an engine-side use-after-free: `Object::callp()`'s debug lock keeps a raw
// `this` and unrefs it after the call. Fixed by https://github.com/godotengine/godot/pull/96856.
#[cfg(since_api = "4.4")]
mod self_destruct_test;
mod singleton_test;
// `validate_property` is only supported in Godot 4.2+.
mod base_init_test;
mod validate_property_test;
mod virtual_methods_niche_test;
mod virtual_methods_test;

#[cfg(feature = "experimental-threads")]
mod thread_safety;

// Need to test this in the init level method.
pub use init_stage_test::*;
