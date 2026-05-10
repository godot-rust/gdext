/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::classes::Engine;
use godot::obj::Singleton;

use crate::framework::itest;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Tests

#[itest(editor)]
fn editor_sanity_check() {
    assert!(
        Engine::singleton().is_editor_hint(),
        "editor-only test must run with editor hint set; invoke via `godot -e --headless`",
    );
}
