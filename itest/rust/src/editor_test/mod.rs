/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// Placeholder substitution for runtime (non-tool) classes is only meaningful since Godot 4.3.
#[cfg(all(since_api = "4.3", feature = "upcoming-editor-placeholders"))]
mod editor_placeholder_test;

mod editor_general_test;

/// On Godot < 4.3, placeholder substitution does not exist; `is_editor_placeholder()` always returns `false`.
#[cfg(all(before_api = "4.3", feature = "upcoming-editor-placeholders"))]
#[crate::framework::itest(editor)]
fn editor_pre_4_3_no_placeholders() {
    use godot::obj::NewGd;
    use godot::register::GodotClass;

    #[derive(GodotClass)]
    #[class(init)]
    struct RuntimeProbe {}

    #[derive(GodotClass)]
    #[class(tool, init)]
    struct ToolProbe {}

    let runtime = RuntimeProbe::new_gd();
    let tool = ToolProbe::new_gd();

    assert!(!runtime.is_editor_placeholder(), "no placeholders (<4.3)");
    assert!(!tool.is_editor_placeholder(), "tool no placeholders (<4.3)");
}
