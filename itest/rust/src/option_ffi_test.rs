/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::prelude::{godot_api, Gd, GodotClass, Node, Object, RefCounted};
use godot::sys::GodotFfi;

use crate::itest;

#[itest]
fn option_some_sys_conversion() {
    let v = Some(Object::new_alloc());
    let ptr = v.sys();

    let v2 = unsafe { Option::<Gd<Object>>::from_sys(ptr) };
    assert_eq!(v2, v);

    v.unwrap().free();
}

#[itest]
fn option_none_sys_conversion() {
    let v = None;
    let ptr = v.sys();

    let v2 = unsafe { Option::<Gd<Object>>::from_sys(ptr) };
    assert_eq!(v2, v);
}

#[derive(GodotClass, Debug)]
#[class(base = RefCounted, init)]
struct OptionFfiTest;

#[godot_api]
impl OptionFfiTest {
    #[func]
    fn return_option_refcounted_none(&self) -> Option<Gd<RefCounted>> {
        None
    }

    #[func]
    fn accept_option_refcounted_none(&self, value: Option<Gd<RefCounted>>) -> bool {
        value.is_none()
    }

    #[func]
    fn return_option_refcounted_some(&self) -> Option<Gd<RefCounted>> {
        Some(RefCounted::new())
    }

    #[func]
    fn accept_option_refcounted_some(&self, value: Option<Gd<RefCounted>>) -> bool {
        value.is_some()
    }

    #[func]
    fn mirror_option_refcounted(&self, value: Option<Gd<RefCounted>>) -> Option<Gd<RefCounted>> {
        value
    }

    #[func]
    fn return_option_node_none(&self) -> Option<Gd<Node>> {
        None
    }

    #[func]
    fn accept_option_node_none(&self, value: Option<Gd<Node>>) -> bool {
        value.is_none()
    }

    #[func]
    fn return_option_node_some(&self) -> Option<Gd<Node>> {
        Some(Node::new_alloc())
    }

    #[func]
    fn accept_option_node_some(&self, value: Option<Gd<Node>>) -> bool {
        value.is_some()
    }

    #[func]
    fn mirror_option_node(&self, value: Option<Gd<Node>>) -> Option<Gd<Node>> {
        value
    }
}
