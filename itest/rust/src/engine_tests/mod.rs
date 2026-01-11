/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

mod async_test;
mod autoload_test;
mod classdb_test;
mod codegen_enums_test;
mod codegen_test;
mod engine_enum_test;
mod gfile_test;
mod match_class_test;
mod native_st_niche_audio_test;
mod native_st_niche_pointer_test;
mod native_structures_test;
mod node_test;
mod save_load_test;
mod translate_test;
mod utilities_test;

pub use classdb_test::check_classdb_full_api;
