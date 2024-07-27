/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::builtin::{GString, PackedStringArray};
use godot::classes::IEditorExportPlugin;
use godot::register::{godot_api, GodotClass};

#[derive(GodotClass)]
#[class(base=EditorExportPlugin, init)]
struct KeywordParameterEditorExportPlugin;

#[godot_api]
impl IEditorExportPlugin for KeywordParameterEditorExportPlugin {
    // This test requires that the second non-self parameter on `export_file`
    // remain named `_type`.
    fn export_file(&mut self, _path: GString, _type: GString, _features: PackedStringArray) {}
}
