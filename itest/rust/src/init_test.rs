/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::prelude::*;

#[derive(GodotClass)]
#[class(init)]
struct WithInitDefaults {
    #[export(get)]
    default_int: i64,

    #[export(get)]
    #[init(default = 42)]
    literal_int: i64,

    #[export(get)]
    #[init(default = -42)]
    expr_int: i64,
}

// TODO Remove once https://github.com/godot-rust/gdext/issues/187 is fixed
#[godot_api]
impl WithInitDefaults {}
