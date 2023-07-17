/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::mem::MaybeUninit;

use godot::prelude::{godot_api, Gd, GodotClass, Node, Object, RefCounted, Share};
use godot::sys;
use godot::sys::{GodotFuncMarshal, PtrcallType};

use crate::itest;

#[itest]
fn option_some_sys_conversion() {
    let v: Option<Gd<Object>> = Some(Object::new_alloc());

    let mut obj: MaybeUninit<Object> = MaybeUninit::uninit();

    // Use `Virtual` ptrcall type since the pointer representation is consistent between godot 4.0 and 4.1.
    unsafe {
        v.share()
            .try_return(
                std::ptr::addr_of_mut!(obj) as sys::GDExtensionTypePtr,
                PtrcallType::Virtual,
            )
            .unwrap()
    };

    let v2 = unsafe {
        Option::<Gd<Object>>::try_from_arg(
            std::ptr::addr_of_mut!(obj) as sys::GDExtensionTypePtr,
            PtrcallType::Virtual,
        )
        .unwrap()
    };

    assert_eq!(v2, v);

    v.unwrap().free();
}

#[itest]
fn option_none_sys_conversion() {
    let v: Option<Gd<Object>> = None;

    let mut obj: MaybeUninit<Object> = MaybeUninit::uninit();

    // Use `Virtual` ptrcall type since the pointer representation is consistent between godot 4.0 and 4.1.
    unsafe {
        v.share()
            .try_return(
                std::ptr::addr_of_mut!(obj) as sys::GDExtensionTypePtr,
                PtrcallType::Virtual,
            )
            .unwrap()
    };

    let v2 = unsafe {
        Option::<Gd<Object>>::try_from_arg(
            std::ptr::addr_of_mut!(obj) as sys::GDExtensionTypePtr,
            PtrcallType::Virtual,
        )
        .unwrap()
    };

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

#[derive(GodotClass)]
#[class(init)]
struct OptionExportFfiTest {
    #[var]
    optional: Option<Gd<Node>>,

    #[export]
    optional_export: Option<Gd<Node>>,
}

#[godot_api]
impl OptionExportFfiTest {}
