/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::framework::itest;
use godot::prelude::*; // Expect match_class! to be in prelude.

// Ensure static types are as expected.
fn require_object(_: &Object) {}
fn require_node(_: &Node) {}
fn require_node2d(_: &Node2D) {}

#[itest]
fn match_class_basic_dispatch() {
    let node2d = Node2D::new_alloc();
    let obj: Gd<Object> = node2d.upcast();
    let to_free = obj.clone();

    let result = match_class!(obj, {
        node @ Node2D => {
            require_node2d(&node);
            1
        },
        node @ Node => {
            require_node(&node);
            2
        },
        _ => 3 // No comma.
    });

    assert_eq!(result, 1);
    to_free.free();
}

#[itest]
fn match_class_shadowed_by_more_general() {
    let node2d = Node2D::new_alloc();
    let obj: Gd<Object> = node2d.upcast();
    let to_free = obj.clone();

    let result = match_class!(obj, {
        _node @ Node => 1,
        _node @ Node2D => 2,
        _ => 3, // Comma.
    });

    assert_eq!(
        result, 1,
        "Node2D branch never hit, since Node one is more general and first"
    );
    to_free.free();
}

#[itest]
fn match_class_ignored_fallback() {
    let obj: Gd<Object> = RefCounted::new_gd().upcast();

    let result = match_class!(obj, {
        _node @ godot::classes::Node => 1, // Test qualified types.
        _res @ Resource => 2,
        _ => 3,
    });

    assert_eq!(result, 3);
}

#[itest]
fn match_class_named_fallback_matched() {
    let obj: Gd<Object> = Resource::new_gd().upcast();

    let result = match_class!(obj, {
        _node @ Node => 1,
        _node @ Node2D => 2,

        // Named fallback with access to original object.
        other @ _ => {
            require_object(&other);
            assert_eq!(other.get_class(), "Resource".into());
            3
        }
    });

    assert_eq!(result, 3);
}

#[itest]
fn match_class_named_fallback_unmatched() {
    // Test complex inline expression.
    let result = match_class!(Resource::new_gd().upcast::<Object>(), {
        _node @ Node => 1,
        _res @ Resource => 2,
        _ignored @ _ => 3,
    });

    assert_eq!(result, 2);
}
