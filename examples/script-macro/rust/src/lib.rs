/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::{classes::ScriptExtension, prelude::*};

struct ScriptMacro;
#[gdextension]
unsafe impl ExtensionLibrary for ScriptMacro {}

#[derive(GodotClass)]
#[class(script, base = ScriptExtension, tool, init)]
struct ExampleScript {
    #[var]
    ex_prop: i32,

    base: Base<ScriptExtension>
}

fn a () {
    dict! {
        "this": ::std::string::String::new()
    };
}