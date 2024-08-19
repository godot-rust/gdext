/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::builtin::{GString, PackedStringArray};
use godot::classes::{IEditorExportPlugin, Node, Resource};
use godot::obj::Gd;
use godot::register::{godot_api, GodotClass};

#[derive(GodotClass)]
#[class(base=EditorExportPlugin, init, tool)]
struct KeywordParameterEditorExportPlugin;

#[godot_api]
#[rustfmt::skip]
impl IEditorExportPlugin for KeywordParameterEditorExportPlugin {
    // This test requires that the second non-self parameter on `export_file`
    // remain named `_type`.
    fn export_file(&mut self, _path: GString, _type: GString, _features: PackedStringArray) {}

    fn customize_resource(&mut self, _resource: Gd<Resource>, _path: GString) -> Option<Gd<Resource>> { unreachable!() }
    fn customize_scene(&mut self, _scene: Gd<Node>, _path: GString) -> Option<Gd<Node>> { unreachable!() }
    fn get_customization_configuration_hash(&self) -> u64 { unreachable!() }
    fn get_name(&self) -> GString { unreachable!() }
}
