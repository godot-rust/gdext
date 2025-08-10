/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::prelude::*;

use crate::framework::itest; // Expect match_class! to be in prelude.

// Ensure static types are as expected.
fn require_object(_: &Object) {}
fn require_mut_object(_: &mut Object) {}
fn require_node(_: &Node) {}
fn require_node2d(_: &Node2D) {}
fn require_mut_node2d(_: &mut Node2D) {}

#[itest]
fn match_class_basic_dispatch() {
    let node2d = Node2D::new_alloc();
    let obj: Gd<Object> = node2d.upcast();
    let to_free = obj.clone();

    let result = match_class! { obj,
        node @ Node2D => {
            require_node2d(&node);
            1
        },
        node @ Node => {
            require_node(&node);
            2
        },
        _ => 3 // No comma.
    };

    assert_eq!(result, 1);
    to_free.free();
}

#[itest]
fn match_class_basic_mut_dispatch() {
    let node2d = Node2D::new_alloc();
    let obj: Gd<Object> = node2d.upcast();
    let to_free = obj.clone();

    let result = match_class! { obj,
        mut node @ Node2D => {
            require_mut_node2d(&mut node);
            1
        },
        node @ Node => {
            require_node(&node);
            2
        },
        _ => 3 // No comma.
    };

    assert_eq!(result, 1);
    to_free.free();
}

#[itest]
fn match_class_basic_unnamed_dispatch() {
    let node3d = Node3D::new_alloc();
    let obj: Gd<Object> = node3d.upcast();
    let to_free = obj.clone();

    let result = match_class! { obj,
        node @ Node2D => {
            require_node2d(&node);
            1
        },
        _ @ Node3D => 2,
        node @ Node => {
            require_node(&node);
            3
        },
        _ => 4 // No comma.
    };

    assert_eq!(result, 2);
    to_free.free();
}

#[itest]
fn match_class_shadowed_by_more_general() {
    let node2d = Node2D::new_alloc();
    let obj: Gd<Object> = node2d.upcast();
    let to_free = obj.clone();

    let result = match_class! { obj,
        _node @ Node => 1,
        _node @ Node2D => 2,
        _ => 3, // Comma.
    };

    assert_eq!(
        result, 1,
        "Node2D branch never hit, since Node one is more general and first"
    );
    to_free.free();
}

#[itest]
fn match_class_ignored_fallback() {
    let obj: Gd<Object> = RefCounted::new_gd().upcast();

    let result = match_class! { obj,
        _node @ godot::classes::Node => 1, // Test qualified types.
        _res @ Resource => 2,
        _ => 3,
    };

    assert_eq!(result, 3);
}

#[itest]
fn match_class_named_fallback_matched() {
    let obj: Gd<Object> = Resource::new_gd().upcast();

    let result = match_class! { obj,
        _node @ Node => 1,
        _node @ Node2D => 2,

        // Named fallback with access to original object.
        other => {
            require_object(&other);
            assert_eq!(other.get_class(), "Resource".into());
            3
        }
    };

    assert_eq!(result, 3);
}

#[itest]
fn match_class_named_mut_fallback_matched() {
    let obj: Gd<Object> = Resource::new_gd().upcast();

    let result = match_class! { obj,
        _node @ Node => 1,
        _node @ Node2D => 2,

        // Named fallback with access to original object.
        mut other => {
            require_mut_object(&mut other);
            assert_eq!(other.get_class(), "Resource".into());
            3
        }
    };

    assert_eq!(result, 3);
}

#[itest]
fn match_class_named_fallback_unmatched() {
    // Test complex inline expression.
    let result = match_class! {
        Resource::new_gd().upcast::<Object>(),
        _node @ Node => 1,
        _res @ Resource => 2,
        _ignored => 3,
    };

    assert_eq!(result, 2);
}

#[itest]
fn match_class_control_flow() {
    let obj: Gd<Object> = Resource::new_gd().upcast();

    let mut broken = false;

    #[expect(clippy::never_loop)]
    for _i in 0..1 {
        let _: i32 = match_class! { obj.clone(),
            _node @ Node => 1,
            _res @ Resource => {
                broken = true;
                break;
            },
            _ => 2
        };

        panic!("break didn't work");
    }

    assert!(broken, "break statement should have been executed");
}

#[itest]
fn match_class_unit_type() {
    let obj: Gd<Object> = Object::new_alloc();
    let to_free = obj.clone();
    let mut val = 0;

    match_class! { obj,
        _ @ Node3D => {
            val = 1;
        },
        mut node @ Node2D => {
            require_mut_node2d(&mut node);
            val = 2;
        },
        node @ Node => {
            require_node(&node);
            val = 3;
        },
        // No need for _ branch since all branches return ().
    }

    assert_eq!(val, 0);
    to_free.free();

    // Special case: no branches at all. Also test unit type.
    let _: () = match_class! { RefCounted::new_gd(),
        // Nothing.
    };
}
