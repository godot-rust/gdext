/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::prelude::*;

#[derive(GodotClass)]
#[class(init)]
pub struct DependencyObj {
    #[var]
    #[init(val = 42)]
    some_property: i64,
}

#[godot_api]
impl DependencyObj {
    #[func]
    fn method_from_dependency(&self) -> u32 {
        42
    }
}

#[allow(unused)]
struct OtherGDExtension;

#[gdextension]
unsafe impl ExtensionLibrary for OtherGDExtension {}
