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
    fn on_stage_init(stage: InitStage) {
        println!("[Rust]      Init stage {stage:?}");
    }

    fn on_stage_deinit(stage: InitStage) {
        println!("[Rust]      Deinit stage {stage:?}");
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[derive(GodotClass)]
#[class(init, base=Node)]
struct Reloadable {
    #[export]
    #[init(val = Planet::Earth)]
    favorite_planet: Planet,

    #[init(val = NoDefault::obtain())]
    _other_object: Gd<NoDefault>,
}

#[godot_api]
impl Reloadable {
    #[func]
    #[rustfmt::skip]
    // DO NOT MODIFY FOLLOWING LINE -- replaced by hot-reload test. Hence #[rustfmt::skip] above.
    fn get_number(&self) -> i64 { 100 }

    #[func]
    fn from_string(s: GString) -> Gd<Self> {
        Gd::from_object(Reloadable {
            favorite_planet: Planet::from_godot(s),
            _other_object: NoDefault::obtain(),
        })
    }
}

// no_init reloadability - https://github.com/godot-rust/gdext/issues/874.
#[derive(GodotClass)]
#[class(no_init, base=Node)]
struct NoDefault {}

#[godot_api]
impl NoDefault {
    #[func]
    fn obtain() -> Gd<Self> {
        Gd::from_object(NoDefault {})
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[derive(Clone, GodotConvert, Var, Export)]
#[godot(via = GString)]
enum Planet {
    Earth,
    Mars,
    Venus,
}
