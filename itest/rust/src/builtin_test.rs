/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::itest;
use godot::builtin::inner::*;
use godot::prelude::*;

#[itest]
fn test_builtins_vector2() {
    let vec = Vector2::new(3.0, -4.0);
    let inner: InnerVector2 = vec.as_inner();

    let len_sq = inner.length_squared();
    assert_eq!(len_sq, 25.0);

    let abs = inner.abs();
    assert_eq!(abs, Vector2::new(3.0, 4.0));

    let normalized = inner.is_normalized();
    assert!(!normalized);
}

#[itest]
fn test_builtins_array() {
    let array = VariantArray::default();
    let mut inner: InnerArray = array.as_inner();

    let a = 7.to_variant();
    let b = GodotString::from("Seven").to_variant();

    inner.append(a.clone());
    inner.append(b.clone());

    assert_eq!(inner.size(), 2);
    assert_eq!(inner.pop_front(), a);
    assert_eq!(inner.pop_front(), b);
    assert_eq!(inner.pop_front(), Variant::nil());
}

#[itest]
fn test_builtins_callable() {
    let obj = Node2D::new_alloc();
    let cb = Callable::from_object_method(obj.share(), "set_position");
    let inner: InnerCallable = cb.as_inner();

    assert!(!inner.is_null());
    assert_eq!(inner.get_object_id(), obj.instance_id().to_i64());
    assert_eq!(inner.get_method(), StringName::from("set_position"));

    // TODO once varargs is available
    // let pos = Vector2::new(5.0, 7.0);
    // inner.call(&[pos.to_variant()]);
    // assert_eq!(obj.get_position(), pos);
    //
    // inner.bindv(array);

    obj.free();
}
