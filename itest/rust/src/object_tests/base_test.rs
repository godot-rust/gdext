/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::framework::itest;
use godot::prelude::*;

#[itest(skip)]
fn base_test_is_weak() {
    // TODO check that Base is a weak pointer (doesn't keep the object alive)
    // This might not be needed, as we have leak detection, but it could highlight regressions faster
}

#[itest]
fn base_instance_id() {
    let obj = Based::alloc_gd();
    let obj_id = obj.instance_id();
    let base_id = obj.bind().base.instance_id();

    assert_eq!(obj_id, base_id);
    obj.free();
}

#[itest]
fn base_access_unbound() {
    let mut obj = Based::alloc_gd();

    let pos = Vector2::new(-5.5, 7.0);
    obj.set_position(pos);
    assert_eq!(obj.get_position(), pos);

    obj.free();
}

// Tests whether access to base is possible from outside the Gd<T>, even if there is no `#[base]` field.
#[itest]
fn base_access_unbound_no_field() {
    let mut obj = Baseless::alloc_gd();

    let pos = Vector2::new(-5.5, 7.0);
    obj.set_position(pos);
    assert_eq!(obj.get_position(), pos);

    obj.free();
}

#[itest]
fn base_display() {
    let obj = Based::alloc_gd();
    {
        let guard = obj.bind();
        let id = guard.base.instance_id();

        // We expect the dynamic type to be part of Godot's to_string(), so Based and not Node2D
        let actual = format!(".:{}:.", guard.base);
        let expected = format!(".:<Based#{id}>:.");

        assert_eq!(actual, expected);
    }
    obj.free();
}

#[itest]
fn base_debug() {
    let obj = Based::alloc_gd();
    {
        let guard = obj.bind();
        let id = guard.base.instance_id();

        // We expect the dynamic type to be part of Godot's to_string(), so Based and not Node2D
        let actual = format!(".:{:?}:.", guard.base);
        let expected = format!(".:Base {{ id: {id}, class: Based }}:.");

        assert_eq!(actual, expected);
    }
    obj.free();
}

#[itest]
fn base_with_init() {
    let obj = Gd::<Based>::from_init_fn(|mut base| {
        base.set_rotation(11.0);
        Based { base, i: 732 }
    });

    {
        let guard = obj.bind();
        assert_eq!(guard.i, 732);
        assert_eq!(guard.base.get_rotation(), 11.0);
    }
    obj.free();
}

#[derive(GodotClass)]
#[class(init, base=Node2D)]
struct Based {
    #[base]
    base: Base<Node2D>,

    i: i32,
}

#[derive(GodotClass)]
#[class(init, base=Node2D)]
struct Baseless {
    // No need for fields, we just test if we can access this as Gd<Node2D>.
}
