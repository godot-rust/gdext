/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::itest;
use godot::prelude::*;

pub(crate) fn run() -> bool {
    let mut ok = true;
    // No tests currently, tests using HasProperty are in Godot scripts.

    ok
}

#[derive(GodotClass)]
#[class(base=Node)]
struct HasProperty {
    #[export(getter = "get_val", setter = "set_val")]
    val: i32,
    #[base]
    base: Base<Node>,
}

#[godot_api]
impl HasProperty {
    #[func]
    pub fn get_val(&self) -> i32 {
        return self.val;
    }

    #[func]
    pub fn set_val(&mut self, val: i32) {
        self.val = val;
    }
}

#[godot_api]
impl GodotExt for HasProperty {
    fn init(base: Base<Node>) -> Self {
        HasProperty { val: 0, base }
    }
}
