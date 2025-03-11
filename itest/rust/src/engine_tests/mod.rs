/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

#[cfg(since_api = "4.2")]
mod async_test;
mod codegen_enums_test;
mod codegen_test;
mod engine_enum_test;
mod gfile_test;
/// Native audio structure tests are only enabled when both the `experimental-threads` and `codegen-full` features are active. The tests
/// require these features to be able to execute.
#[cfg(all(feature = "experimental-threads", feature = "codegen-full"))]
mod native_audio_structures_test;
mod native_structures_test;
mod node_test;
mod save_load_test;
mod translate_test;
mod utilities_test;
