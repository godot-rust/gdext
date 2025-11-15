/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::prelude::*;

use crate::framework::itest;

const REF_COUNTED_ICON: &str = "res://icons/ref_counted_icon.svg";
#[derive(GodotClass)]
#[class(init, base=RefCounted, icon = REF_COUNTED_ICON)]
struct ClassWithIconRefCounted {
    base: Base<RefCounted>,
}

#[derive(GodotClass)]
#[class(init, base=Node, icon = "INVALID!")]
struct ClassWithInvalidIconNode {
    base: Base<Node>,
}

#[derive(GodotClass)]
#[class(init, base=Node, icon = "res://icons/node_icon.svg")]
struct ClassWithIconNode {
    base: Base<Node>,
}

#[derive(GodotClass)]
#[class(init, tool, base=RefCounted, icon = "res://icons/tool_icon.svg")]
struct ToolClassWithIcon {
    base: Base<RefCounted>,
}

#[derive(GodotClass)]
#[class(init, base=RefCounted, icon = "res://icons/another_icon.svg")]
struct AnotherClassWithIcon {
    base: Base<RefCounted>,
}

#[itest]
fn class_with_icon_refcounted_registers() {
    let instance = ClassWithIconRefCounted::new_gd();
    assert!(instance.is_instance_valid());
}

#[itest]
fn class_with_invalid_icon_refcounted_registers() {
    let instance = ClassWithInvalidIconNode::new_alloc();
    assert!(instance.is_instance_valid());
}

#[itest]
fn class_with_icon_node_registers() {
    let instance = ClassWithIconNode::new_alloc();
    assert!(instance.is_instance_valid());
    instance.free();
}

#[itest]
fn tool_class_with_icon_registers() {
    let instance = ToolClassWithIcon::new_gd();
    assert!(instance.is_instance_valid());
}

#[itest]
fn multiple_classes_with_different_icons_register() {
    let instance1 = ClassWithIconRefCounted::new_gd();
    let instance2 = AnotherClassWithIcon::new_gd();

    assert!(instance1.is_instance_valid());
    assert!(instance2.is_instance_valid());
}
