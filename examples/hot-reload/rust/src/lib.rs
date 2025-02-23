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
    base: Base<Node>,
}

#[godot_api]
impl INode for Reloadable {
    fn init(base: Base<Self::Base>) -> Self {
        // HOT-RELOAD: change values to initialize with different defaults.
        Self {
            favorite_planet: Planet::Earth,
            //some_string: "Hello, world!".into(),
            base,
        }
    }

    fn ready(&mut self) {
        let tree = self.base().get_tree().unwrap();

        godot_print!("starting async task!");

        godot_task(async move {
            let signal = Signal::from_object_signal(&tree, "physics_frame");

            loop {
                let result = signal.to_try_future::<()>().await;

                godot_print!("[Async Task] signal result: {:?}", result);
            }
        });
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
        Gd::from_init_fn(|base| Reloadable {
            favorite_planet: Planet::from_godot(s),
            base,
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
