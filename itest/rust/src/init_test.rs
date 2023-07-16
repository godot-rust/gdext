/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::itest;
use godot::prelude::*;

#[derive(GodotClass)]
#[class(init)]
struct WithInitDefaults {
    #[var(get)]
    default_int: i64,

    #[var(get)]
    #[init(default = 42)]
    literal_int: i64,

    #[var(get)]
    #[init(default = -42)]
    expr_int: i64,
}

// TODO Remove once https://github.com/godot-rust/gdext/issues/187 is fixed
#[godot_api]
impl WithInitDefaults {}

#[itest]
fn cfg_test() {
    // Makes sure that since_api and before_api are mutually exclusive
    assert_ne!(cfg!(since_api = "4.1"), cfg!(before_api = "4.1"));
    assert_ne!(cfg!(since_api = "4.2"), cfg!(before_api = "4.2"));
}
