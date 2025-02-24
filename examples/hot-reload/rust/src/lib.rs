#![cfg_attr(published_docs, feature(doc_cfg))]
/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::prelude::*;

struct HotReload;

#[gdextension]
unsafe impl ExtensionLibrary for HotReload {
    fn on_level_init(_level: InitLevel) {
        println!("[Rust]      Init level {:?}", _level);
    }

    fn on_level_deinit(_level: InitLevel) {
        println!("[Rust]      Deinit level {:?}", _level);
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[derive(GodotClass)]
#[class(base=Node)]
struct Reloadable {
    #[export]
    favorite_planet: Planet,
    //
    // HOT-RELOAD: uncomment this to add a new exported field (also update init() below).
    // #[export]
    // some_string: GString,
}

#[godot_api]
impl INode for Reloadable {
    fn init(_base: Base<Self::Base>) -> Self {
        // HOT-RELOAD: change values to initialize with different defaults.
        Self {
            favorite_planet: Planet::Earth,
            //some_string: "Hello, world!".into(),
        }
    }
}

#[godot_api]
impl Reloadable {
    #[rustfmt::skip] // easier replacement by test.
    #[func]
    // HOT-RELOAD: change returned value for dynamic code change.
    fn get_number(&self) -> i64 { 100 }

    #[func]
    fn from_string(s: GString) -> Gd<Self> {
        Gd::from_object(Reloadable {
            favorite_planet: Planet::from_godot(s),
        })
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[derive(GodotConvert, Var, Export)]
#[godot(via = GString)]
enum Planet {
    Earth,
    Mars,
    Venus,
    //
    // HOT-RELOAD: uncomment this to extend enum.
    //Jupiter,
}
