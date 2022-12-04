/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::path::Path;

fn main() {
    godot_codegen::generate_core_files(Path::new(concat!(
        //
        env!("CARGO_MANIFEST_DIR"),
        "/src/gen/"
    )));
}
