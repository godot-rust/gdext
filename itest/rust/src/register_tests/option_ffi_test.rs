/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::builtin::NodePath;
use godot::classes::{Node, Object, RefCounted, Resource};
use godot::meta::{FromGodot, GodotType, ToGodot};
use godot::obj::{Gd, NewAlloc, NewGd, RawGd};
use godot::register::{GodotClass, godot_api};
use godot::sys::GodotFfi;

use crate::framework::itest;

#[itest]
fn option_some_sys_conversion() {
    let v = Some(Object::new_alloc());
    let v_raw = v.to_ffi();
    let ptr = v_raw.sys();

    let v2_raw = unsafe { RawGd::<Object>::new_from_sys(ptr) };
    let v2 = Option::<Gd<Object>>::from_ffi(v2_raw);
    assert_eq!(v2, v);

    // We're testing this behavior.
    #[allow(clippy::unnecessary_literal_unwrap)]
    v.unwrap().free();
}

#[itest]
fn option_none_sys_conversion() {
    let v: Option<Gd<Object>> = None;
    let v_raw = v.to_ffi();
    let ptr = v_raw.sys();

    let v2_raw = unsafe { RawGd::<Object>::new_from_sys(ptr) };
    let v2 = Option::<Gd<Object>>::from_ffi(v2_raw);
    assert_eq!(v2, v);
}

// Godot 4.2's inspector "clear" button on an #[export]ed Option<Gd<T>> property sends an empty NodePath instead of nil; both conversion
// paths must yield None. See Gd::qualifies_as_special_none().
#[itest]
fn option_special_none_from_empty_node_path() {
    let empty = NodePath::default().to_variant();
    assert_eq!(Option::<Gd<Node>>::try_from_variant(&empty).unwrap(), None);
    assert_eq!(Option::<Gd<Node>>::from_variant(&empty), None);

    // Non-empty NodePath is not special-cased and must not yield None.
    let non_empty = NodePath::from("Some/Path").to_variant();
    assert!(Option::<Gd<Node>>::try_from_variant(&non_empty).is_err());
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
        Some(RefCounted::new_gd())
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
    optional_export: Option<Gd<Resource>>,
}
