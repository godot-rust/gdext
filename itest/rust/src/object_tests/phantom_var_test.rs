/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::register::property::PhantomVar;
use godot::register::{godot_api, GodotClass};

#[derive(GodotClass)]
#[class(init)]
struct HasPhantomVar {
    #[var(get = get_read_only)]
    read_only: PhantomVar<i64>,

    #[var(get = get_read_write, set = set_read_write)]
    read_write: PhantomVar<i64>,

    value: i64,
}

#[godot_api]
impl HasPhantomVar {
    #[func]
    fn get_read_only(&self) -> i64 {
        self.value
    }

    #[func]
    fn get_read_write(&self) -> i64 {
        self.value
    }

    #[func]
    fn set_read_write(&mut self, value: i64) {
        self.value = value;
    }
}
