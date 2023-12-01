/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

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
