/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::prelude::*;

use crate::framework::itest;

const ICON: &str = "res://icons/icon.svg";
#[derive(GodotClass)]
#[class(init, base=RefCounted, icon = ICON)]
struct ClassWithIconRefCounted {
    base: Base<RefCounted>,
}

#[derive(GodotClass)]
#[class(init, base=Node, tool, icon = ICON)]
struct ClassWithIconNode {
    base: Base<Node>,
}

#[itest]
fn class_icon_registers() {
    let instance1 = ClassWithIconRefCounted::new_gd();
    let instance2 = ClassWithIconNode::new_alloc();

    assert!(instance1.is_instance_valid());
    assert!(instance2.is_instance_valid());

    instance2.free();
}
